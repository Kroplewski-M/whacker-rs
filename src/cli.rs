#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(short, long)]
    pub url: String,
    #[arg(short, long, default_value_t = 30, value_parser = clap::value_parser!(u16).range(1..))]
    pub seconds: u16,
    #[arg(short, long, value_parser = clap::value_parser!(u16).range(1..))]
    pub threads: Option<u16>,
    #[arg(short, long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..))]
    pub connections: u16,
}
