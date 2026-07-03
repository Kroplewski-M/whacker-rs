use http_body_util::Empty;
use hyper_util::rt::TokioIo;
use std::time::{Duration, Instant};

use crate::cli::Args;
use crate::connection::Conn;
use crate::request;

pub async fn run(args: Args) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let deadline = Instant::now() + Duration::from_secs(args.seconds as u64);
    let mut handles = Vec::new();
    let scheme = args.url.scheme_str().unwrap_or("http");
    let tls_config = Conn::build_tls_config(scheme);

    for _ in 0..args.connections {
        let connection = Conn::new(args.url.clone(), tls_config.clone());
        let io = connection.connect().await?;
        let (mut sender, conn) =
            hyper::client::conn::http1::handshake::<_, Empty<hyper::body::Bytes>>(TokioIo::new(io))
                .await?;

        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                eprintln!("Connection failed: {:?}", err);
            }
        });

        handles.push(tokio::task::spawn(async move {
            while Instant::now() < deadline {
                request::send_request(&connection.url, &mut sender).await?;
            }
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        }));
    }

    for h in handles {
        h.await??;
    }
    Ok(())
}
