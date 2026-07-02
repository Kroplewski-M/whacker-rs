use crate::cli::Args;
use clap::Parser;

mod cli;
mod connection;
mod request;
mod worker;

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
        .block_on(worker::run(args))
}
