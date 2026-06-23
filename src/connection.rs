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

pub struct Conn {
    pub url: hyper::Uri,
    pub host: String,
    pub tls_config: Option<Arc<ClientConfig>>,
}
impl Conn {
    pub fn new(url: hyper::Uri) -> Self {
        let scheme = url.scheme_str().unwrap_or("http").to_owned();
        let host = url.host().expect("uri has no host").to_owned();

        let mut config: Option<ClientConfig> = None;
        if scheme == "https" {
            let mut root_store = RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            config = Some(
                ClientConfig::builder_with_provider(Arc::new(
                    tokio_rustls::rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_root_certificates(root_store)
                .with_no_client_auth(),
            );
        }
        let tls_config = config.map(Arc::new);

        Self {
            url,
            host,
            tls_config,
        }
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
