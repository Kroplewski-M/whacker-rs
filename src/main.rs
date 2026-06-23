use clap::Parser;
use http_body_util::{BodyExt, Empty};
use hyper::{Request, body::Bytes};
use hyper_util::rt::TokioIo;
use tokio::io::AsyncWriteExt;

use crate::cli::Args;
use crate::connection::Conn;

mod cli;
mod connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    let url = args.url.parse::<hyper::Uri>()?;

    let connection = Conn::new(url);
    let io = connection.connect().await?;

    let (mut sender, conn) =
        hyper::client::conn::http1::handshake::<_, Empty<Bytes>>(TokioIo::new(io)).await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

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
