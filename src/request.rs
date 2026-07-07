use std::time::Instant;

use http_body_util::{BodyExt, Empty};
use hyper::client::conn::http1::SendRequest;
use hyper::{Request, body::Bytes};

use crate::metrics::RequestMetric;

pub async fn send_request(
    url: &hyper::Uri,
    sender: &mut SendRequest<Empty<Bytes>>,
) -> RequestMetric {
    let start = Instant::now();
    let mut status_code = None;
    let mut bytes_received = 0;

    let outcome: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
        let authority = url.authority().ok_or("missing authority")?.clone();
        let req = Request::builder()
            .uri(url)
            .header(hyper::header::HOST, authority.as_str())
            .body(Empty::<Bytes>::new())?;

        let mut res = sender.send_request(req).await?;
        status_code = Some(res.status());
        while let Some(next) = res.frame().await {
            let frame = next?;
            if let Some(chunk) = frame.data_ref() {
                bytes_received += chunk.len();
            }
        }
        Ok(())
    }
    .await;
    let metric = RequestMetric {
        status_code,
        bytes_received,
        duration: start.elapsed(),
        error: outcome.err().map(|e| e.to_string()),
    };

    println!("{:?}", metric);
    metric
}
