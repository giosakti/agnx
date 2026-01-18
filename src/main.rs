mod build_info;
mod config;
mod handlers;
mod response;

use axum::routing::get;
use axum::Router;
use clap::Parser;
use config::Config;
use std::net::{IpAddr, SocketAddr};

/// Agnx - A minimal and fast self-hosted runtime for durable and portable AI agents
#[derive(Parser, Debug)]
#[command(version = build_info::VERSION_STRING, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Port to listen on (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut config = Config::load(&args.config)?;

    // CLI port overrides config
    if let Some(port) = args.port {
        config.port = port;
    }

    let app = Router::new()
        .route("/livez", get(handlers::livez))
        .route("/readyz", get(handlers::readyz))
        .route("/version", get(handlers::version))
        .route("/example-bad-request", get(handlers::example_bad_request))
        .route("/example-not-found", get(handlers::example_not_found))
        .route(
            "/example-internal-error",
            get(handlers::example_internal_error),
        );

    let ip: IpAddr = config.host.parse()?;
    let addr = SocketAddr::new(ip, config.port);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("Starting server on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
