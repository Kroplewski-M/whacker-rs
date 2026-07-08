# whacker-rs

A simple HTTP load-testing CLI written in Rust, built on `hyper`, `tokio`, and `tokio-rustls`.

It opens a pool of persistent HTTP/1.1 connections spread across worker threads and hammers a
target URL for a fixed duration, then reports latency, throughput, and status code metrics.

## Usage

```sh
cargo run --release -- --url https://example.com
```

### Options

| Flag            | Short | Default                     | Description                                                              |
| --------------- | ----- | --------------------------- | ------------------------------------------------------------------------ |
| `--url`         | `-u`  | _(required)_                | Target URL to send requests to                                           |
| `--seconds`     | `-s`  | `30`                        | How long to run the test, in seconds                                     |
| `--threads`     | `-t`  | number of available threads | Number of OS threads driving the load                                    |
| `--connections` | `-c`  | `50`                        | Number of persistent connections opened (one worker task per connection) |

Example — hit a local server for 10 seconds over 4 threads with 100 connections:

```sh
cargo run --release -- --url http://localhost:8080 --seconds 10 --threads 4 --connections 100
```

## Output

At the end of the run, `whacker-rs` prints a summary including:

- Total requests sent and success rate
- Requests per second
- Total bytes received
- Average, min, and max request duration
- Latency percentiles (p50, p95, p99)
- A breakdown of status codes received
- Any errors encountered

## How it works

- `cli.rs` — command-line argument parsing (`clap`)
- `connection.rs` — opens a raw TCP connection, optionally wrapped in TLS via `tokio-rustls`
- `request.rs` — sends a single HTTP request over a connection and records its metrics
- `worker.rs` — spawns one async task per connection, each looping requests until the deadline
- `metrics.rs` — aggregates and prints the final report

## Building

```sh
cargo build --release
```

## Testing

```sh
cargo test
```
