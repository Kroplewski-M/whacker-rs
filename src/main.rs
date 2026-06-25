use std::time::{Duration, Instant};

use clap::Parser;
use http_body_util::{BodyExt, Empty};
use hyper::client::conn::http1::SendRequest;
use hyper::{Request, body::Bytes};
use hyper_util::rt::TokioIo;
use tokio::io::AsyncWriteExt;

use crate::cli::Args;
use crate::connection::Conn;

mod cli;
mod connection;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = Args::parse();
    if args.threads.is_none() {
        if let Ok(num) = std::thread::available_parallelism() {
            args.threads = Some(num.get() as u16);
        } else {
            panic!(
                "number of threads not selected and unable to get the number of threads available"
            );
        }
    }
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads.unwrap() as usize)
        .enable_all()
        .build()?
        .block_on(run(args))
}
async fn run(args: Args) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = args.url.parse::<hyper::Uri>()?;
    let deadline = Instant::now() + Duration::from_secs(args.seconds as u64);
    let mut handles = Vec::new();

    for _ in 0..args.connections {
        let connection = Conn::new(url.clone());
        let io = connection.connect().await?;
        let (mut sender, conn) =
            hyper::client::conn::http1::handshake::<_, Empty<Bytes>>(TokioIo::new(io)).await?;

        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        handles.push(tokio::task::spawn(async move {
            while Instant::now() < deadline {
                send_request(&connection, &mut sender).await?;
            }
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        }));
    }

    for h in handles {
        h.await??;
    }
    Ok(())
}

async fn send_request(
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
