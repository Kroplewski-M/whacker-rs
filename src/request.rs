use http_body_util::{BodyExt, Empty};
use hyper::client::conn::http1::SendRequest;
use hyper::{Request, body::Bytes};
use tokio::io::AsyncWriteExt;

use crate::connection::Conn;

pub async fn send_request(
    connection: &Conn,
    sender: &mut SendRequest<Empty<Bytes>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let authority = &connection.url.authority().unwrap().clone();

    let req = Request::builder()
        .uri(&connection.url)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    let mut res = sender.send_request(req).await?;
    println!("Response status: {}", res.status());

    while let Some(next) = res.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            tokio::io::stdout().write_all(chunk).await?;
        }
    }
    Ok(())
}
