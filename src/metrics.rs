use std::collections::HashMap;
use std::time::Duration;

use hyper::StatusCode;

#[derive(Debug)]
pub struct RequestMetric {
    pub status_code: Option<StatusCode>,
    pub duration: Duration,
    pub bytes_received: usize,
    pub error: Option<String>,
}

fn percentile(sorted_durations: &[Duration], p: f64) -> Duration {
    let rank = (p / 100.0 * (sorted_durations.len() - 1) as f64).round() as usize;
    sorted_durations[rank]
}

pub fn log_metrics(metrics: Vec<RequestMetric>, elapsed: Duration) {
    if metrics.is_empty() {
        println!("No requests were sent.");
        return;
    }

    let mut durations = metrics.iter().map(|x| x.duration).collect::<Vec<_>>();
    durations.sort();

    let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
    let min_duration = durations[0];
    let max_duration = durations[durations.len() - 1];
    let p50 = percentile(&durations, 50.0);
    let p95 = percentile(&durations, 95.0);
    let p99 = percentile(&durations, 99.0);

    let total_bytes = metrics.iter().map(|x| x.bytes_received).sum::<usize>();

    let error_count = metrics.iter().filter(|x| x.error.is_some()).count();
    let success_rate = (metrics.len() - error_count) as f64 / metrics.len() as f64 * 100.0;
    let requests_per_sec = metrics.len() as f64 / elapsed.as_secs_f64();

    let status_code_counts = metrics.iter().filter_map(|x| x.status_code).fold(
        HashMap::<StatusCode, usize>::new(),
        |mut acc, code| {
            *acc.entry(code).or_insert(0) += 1;
            acc
        },
    );

    println!("==================");
    println!("Metrics");
    println!("==================");
    println!("Requests Sent: {}", metrics.len());
    println!("Success Rate: {:.2}%", success_rate);
    println!("Requests/sec: {:.2}", requests_per_sec);
    println!("Total Bytes Received: {}", total_bytes);
    println!(
        "Duration: avg {}ms, min {}ms, max {}ms",
        avg_duration.as_millis(),
        min_duration.as_millis(),
        max_duration.as_millis()
    );
    println!(
        "Latency Percentiles: p50 {}ms, p95 {}ms, p99 {}ms",
        p50.as_millis(),
        p95.as_millis(),
        p99.as_millis()
    );
    println!("Status Codes Received:");
    for code in status_code_counts {
        println!("- {} : {} times", code.0, code.1);
    }
    if error_count > 0 {
        println!("Errors received:");
        for err in metrics.iter().filter_map(|x| x.error.as_ref()) {
            println!("- {}", err);
        }
    }
}
