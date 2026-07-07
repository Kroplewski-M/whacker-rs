use hyper::StatusCode;

#[derive(Debug)]
pub struct RequestMetric {
    pub status_code: Option<StatusCode>,
    pub duration: std::time::Duration,
    pub bytes_received: usize,
    pub error: Option<String>,
}
