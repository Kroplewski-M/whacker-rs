use std::sync::Arc;
use tokio::net::TcpStream;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::{
    TlsConnector,
    rustls::{ClientConfig, RootCertStore, pki_types::ServerName},
};

// Rust only allows one non-auto trait in a dyn object, so combine AsyncRead+AsyncWrite here
pub trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncStream for T {}

#[derive(Debug, Clone)]
pub struct Conn {
    pub url: hyper::Uri,
    pub host: String,
    pub tls_config: Option<Arc<ClientConfig>>,
}
impl Conn {
    pub fn new(url: hyper::Uri, tls_config: Option<Arc<ClientConfig>>) -> Self {
        let host = url.host().expect("uri has no host").to_owned();
        Self {
            url,
            host,
            tls_config,
        }
    }
    pub fn build_tls_config(scheme: impl Into<String> + PartialEq) -> Option<Arc<ClientConfig>> {
        let scheme = scheme.into();
        if scheme != "https" {
            return None;
        }
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder_with_provider(Arc::new(
            tokio_rustls::rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();

        Some(Arc::new(config))
    }
    pub async fn connect(
        &self,
    ) -> Result<Box<dyn AsyncStream>, Box<dyn std::error::Error + Send + Sync>> {
        // Open raw TCP connection

        let port = self
            .url
            .port_u16()
            .unwrap_or(if self.tls_config.is_some() { 443 } else { 80 });

        let address = format!("{}:{}", self.host, port);

        let stream = TcpStream::connect(address).await?;

        // For HTTPS: perform TLS handshake manually with tokio-rustls, then box both
        // paths to the same dyn AsyncStream so handshake sees one concrete type
        if let Some(tls_config) = &self.tls_config {
            let connector = TlsConnector::from(tls_config.clone());
            let server_name = ServerName::try_from(self.host.to_owned())?;

            Ok(Box::new(connector.connect(server_name, stream).await?) as Box<dyn AsyncStream>)
        } else {
            Ok(Box::new(stream) as Box<dyn AsyncStream>)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn build_tls_config_http() {
        let config = Conn::build_tls_config("http");
        assert!(config.is_none());
    }
    #[test]
    fn build_tls_config_none() {
        let config = Conn::build_tls_config("");
        assert!(config.is_none());
    }
    #[test]
    fn build_tls_config_random() {
        let config = Conn::build_tls_config("random");
        assert!(config.is_none());
    }
    #[test]
    fn build_tls_config_https() {
        let config = Conn::build_tls_config("https");
        assert!(config.is_some());
    }
    #[test]
    fn build_tls_config_uppercase_https_is_not_matched() {
        // Documents current case-sensitive behavior: scheme_str() from hyper::Uri is
        // preserved as-written, so an uppercase scheme falls through to plaintext.
        let config = Conn::build_tls_config("HTTPS");
        assert!(config.is_none());
    }

    #[test]
    fn new_extracts_host_without_port() {
        let uri: hyper::Uri = "http://example.com:8080/path".parse().unwrap();
        let conn = Conn::new(uri, None);
        assert_eq!(conn.host, "example.com");
    }

    #[test]
    fn new_stores_tls_config() {
        let uri: hyper::Uri = "https://example.com".parse().unwrap();
        let tls_config = Conn::build_tls_config("https");
        let conn = Conn::new(uri, tls_config.clone());
        assert!(conn.tls_config.is_some());

        let uri: hyper::Uri = "http://example.com".parse().unwrap();
        let conn = Conn::new(uri, None);
        assert!(conn.tls_config.is_none());
    }

    #[test]
    #[should_panic(expected = "uri has no host")]
    fn new_panics_when_uri_has_no_host() {
        let uri: hyper::Uri = "/path".parse().unwrap();
        Conn::new(uri, None);
    }

    #[tokio::test]
    async fn connect_plaintext_reads_and_writes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 5];
            socket.read_exact(&mut buf).await.unwrap();
            socket.write_all(b"world").await.unwrap();
        });

        let uri: hyper::Uri = format!("http://{}", addr).parse().unwrap();
        let conn = Conn::new(uri, None);
        let mut stream = conn.connect().await.unwrap();

        stream.write_all(b"hello").await.unwrap();
        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"world");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn connect_tls_fails_against_non_tls_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let _ = listener.accept().await.unwrap();
        });

        let uri: hyper::Uri = format!("https://localhost:{}", addr.port())
            .parse()
            .unwrap();
        let tls_config = Conn::build_tls_config("https");
        let conn = Conn::new(uri, tls_config);

        let result = conn.connect().await;
        assert!(result.is_err());

        server.await.unwrap();
    }

    #[tokio::test]
    async fn connect_fails_when_connection_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener); // free the port so nothing is listening on it

        let uri: hyper::Uri = format!("http://{}", addr).parse().unwrap();
        let conn = Conn::new(uri, None);

        assert!(conn.connect().await.is_err());
    }
}
