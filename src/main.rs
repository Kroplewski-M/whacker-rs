use http_body_util::{BodyExt, Empty};
use hyper::{Request, body::Bytes};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

// Rust only allows one non-auto trait in a dyn object, so combine AsyncRead+AsyncWrite here
trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncStream for T {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = "https://v2.jokeapi.dev/joke/Any".parse::<hyper::Uri>()?;

    let scheme = url.scheme_str().unwrap_or("http");
    let host = url.host().expect("uri has no host");
    let port = url
        .port_u16()
        .unwrap_or(if scheme == "https" { 443 } else { 80 });
    let address = format!("{}:{}", host, port);

    // Open raw TCP connection
    let stream = TcpStream::connect(address).await?;

    // For HTTPS: perform TLS handshake manually with tokio-rustls, then box both
    // paths to the same dyn AsyncStream so handshake sees one concrete type
    let io: Box<dyn AsyncStream> = if scheme == "https" {
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let tls_config = ClientConfig::builder_with_provider(Arc::new(
            tokio_rustls::rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()?
        .with_root_certificates(root_store)
        .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(tls_config));
        let server_name = ServerName::try_from(host.to_owned())?;
        Box::new(connector.connect(server_name, stream).await?)
    } else {
        Box::new(stream)
    };

    let (mut sender, conn) =
        hyper::client::conn::http1::handshake::<_, Empty<Bytes>>(TokioIo::new(io)).await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let authority = url.authority().unwrap().clone();

    let req = Request::builder()
        .uri(url)
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
