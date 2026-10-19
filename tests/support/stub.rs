//! A stand-in for a unit's HTTP service: it keeps every request it is asked and answers each path as a test
//! sets it, until it is dropped.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse;
use serde_json::Value;

/// One request the stand-in was asked.
#[derive(Debug, Clone)]
pub struct Asked {
  pub path: String,
  pub authorization: Option<String>,
  pub body: Value,
}

pub struct Stub {
  pub address: String,
  asked: Arc<Mutex<Vec<Asked>>>,
  stop: Option<tokio::sync::oneshot::Sender<()>>,
  serving: Option<std::thread::JoinHandle<()>>,
}

impl Stub {
  /// A stand-in answering each path with its status and JSON body, and any other with 404.
  pub fn answering(answers: &[(&str, u16, Value)]) -> Self {
    let answers: Arc<BTreeMap<String, (u16, Value)>> = Arc::new(
      answers
        .iter()
        .map(|(path, status, body)| ((*path).to_owned(), (*status, body.clone())))
        .collect(),
    );
    let asked = Arc::new(Mutex::new(Vec::new()));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let kept = Arc::clone(&asked);

    listener.set_nonblocking(true).unwrap();

    let serving = std::thread::spawn(move || {
      let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

      runtime.block_on(async move {
        let listener = tokio::net::TcpListener::from_std(listener).unwrap();
        let app = Router::new().fallback(move |uri: Uri, headers: HeaderMap, body: Bytes| {
          let answers = Arc::clone(&answers);
          let kept = Arc::clone(&kept);

          async move {
            kept.lock().unwrap().push(Asked {
              path: uri.path().to_owned(),
              authorization: headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
              body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            });

            match answers.get(uri.path()) {
              Some((status, Value::Null)) => StatusCode::from_u16(*status).unwrap().into_response(),
              Some((status, body)) => {
                (StatusCode::from_u16(*status).unwrap(), axum::Json(body.clone())).into_response()
              }
              None => StatusCode::NOT_FOUND.into_response(),
            }
          }
        });

        axum::serve(listener, app)
          .with_graceful_shutdown(async {
            let _ = stopped.await;
          })
          .await
          .unwrap();
      });
    });

    Self {
      address,
      asked,
      stop: Some(stop),
      serving: Some(serving),
    }
  }

  /// Every request asked so far, in order.
  pub fn asked(&self) -> Vec<Asked> {
    self.asked.lock().unwrap().clone()
  }
}

impl Drop for Stub {
  fn drop(&mut self) {
    if let Some(stop) = self.stop.take() {
      let _ = stop.send(());
    }

    if let Some(serving) = self.serving.take() {
      let _ = serving.join();
    }
  }
}
