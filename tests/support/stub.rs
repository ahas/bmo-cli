//! A stand-in for a unit's HTTP service, on a loopback port or a Unix socket: it keeps every request it is
//! asked and answers each path as a test sets it, until it is dropped.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::IntoResponse;
use serde_json::Value;

/// One request the stand-in was asked.
#[derive(Debug, Clone)]
pub struct Asked {
  pub method: String,
  pub path: String,
  pub authorization: Option<String>,
  pub body: Value,
}

pub struct Stub {
  /// The base address of a stand-in on a port; empty for one on a socket.
  pub address: String,
  asked: Arc<Mutex<Vec<Asked>>>,
  stop: Option<tokio::sync::oneshot::Sender<()>>,
  serving: Option<std::thread::JoinHandle<()>>,
}

// Where the stand-in listens.
enum Listening {
  Port(std::net::TcpListener),
  Socket(std::os::unix::net::UnixListener),
}

impl Stub {
  /// A stand-in on a loopback port answering each path with its status and JSON body, and any other with 404.
  pub fn answering(answers: &[(&str, u16, Value)]) -> Self {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());

    Self::serve(Listening::Port(listener), address, answers)
  }

  /// A stand-in on the Unix socket `socket`, answering as `answering` does.
  pub fn answering_on(socket: &Path, answers: &[(&str, u16, Value)]) -> Self {
    let listener = std::os::unix::net::UnixListener::bind(socket).unwrap();

    Self::serve(Listening::Socket(listener), String::new(), answers)
  }

  fn serve(listening: Listening, address: String, answers: &[(&str, u16, Value)]) -> Self {
    let answers: Arc<BTreeMap<String, (u16, Value)>> = Arc::new(
      answers
        .iter()
        .map(|(path, status, body)| ((*path).to_owned(), (*status, body.clone())))
        .collect(),
    );
    let asked = Arc::new(Mutex::new(Vec::new()));
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let kept = Arc::clone(&asked);
    let app = Router::new().fallback(move |method: Method, uri: Uri, headers: HeaderMap, body: Bytes| {
      let answers = Arc::clone(&answers);
      let kept = Arc::clone(&kept);

      async move {
        kept.lock().unwrap().push(Asked {
          method: method.to_string(),
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
    let stopping = async {
      let _ = stopped.await;
    };
    let serving = std::thread::spawn(move || {
      let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

      runtime.block_on(async move {
        match listening {
          Listening::Port(listener) => {
            listener.set_nonblocking(true).unwrap();
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();

            axum::serve(listener, app).with_graceful_shutdown(stopping).await.unwrap();
          }
          Listening::Socket(listener) => {
            listener.set_nonblocking(true).unwrap();
            let listener = tokio::net::UnixListener::from_std(listener).unwrap();

            axum::serve(listener, app).with_graceful_shutdown(stopping).await.unwrap();
          }
        }
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
