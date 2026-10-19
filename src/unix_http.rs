//! One HTTP/1.1 request to a service listening on a Unix socket, its answer read as JSON.

use std::path::Path;

use http_body_util::{BodyExt as _, Full};
use hyper::body::Bytes;
use hyper::{Method, Request, header};
use hyper_util::rt::TokioIo;
use serde_json::Value;
use tokio::net::UnixStream;

/// `method` on `path` of the service at `socket`, with `body` as JSON where one is given: the status and the
/// JSON it answered with, `null` for an answer that is no JSON; a socket that cannot be reached is the error.
pub fn request(
  socket: &Path,
  method: Method,
  path: &str,
  body: Option<&Value>,
) -> std::io::Result<(u16, Value)> {
  let runtime = tokio::runtime::Builder::new_current_thread().enable_io().build()?;

  runtime.block_on(async {
    let stream = UnixStream::connect(socket).await?;
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
      .await
      .map_err(std::io::Error::other)?;

    tokio::spawn(connection);

    let mut asked = Request::builder().method(method).uri(path).header(header::HOST, "localhost");
    let bytes = match body {
      Some(body) => {
        asked = asked.header(header::CONTENT_TYPE, "application/json");
        Bytes::from(serde_json::to_vec(body)?)
      }
      None => Bytes::new(),
    };
    let answer = sender
      .send_request(asked.body(Full::new(bytes)).map_err(std::io::Error::other)?)
      .await
      .map_err(std::io::Error::other)?;
    let status = answer.status().as_u16();
    let said = answer.into_body().collect().await.map_err(std::io::Error::other)?.to_bytes();

    Ok((status, serde_json::from_slice(&said).unwrap_or(Value::Null)))
  })
}
