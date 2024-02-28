use std::env::*;
use std::net::TcpListener;
use std::os::unix::io::FromRawFd;
use tide_tracing::TraceMiddleware;
use tracing::log::*;

mod api;
mod dal;
mod error;
mod fusion;
mod index;
#[cfg(feature = "integration")]
mod test_utils;

//use core::slice::SlicePattern;
#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        //.without_time()
        .init();

    info!("Starting brutus");
    let mut app = tide::new();

    app.with(TraceMiddleware::new());

    app.at("/")
        .get(|_| async { Ok(format!("brutus v{}", env!("CARGO_PKG_VERSION"))) });
    app.at("/api/v1/").nest(api::routes()?);
    app.at("/docs")
        .get(tide::Redirect::new("/docs/ui/index.html"));
    app.at("/docs/").serve_dir("docs/")?;

    let bind_to = var("BIND_TO").unwrap_or("0.0.0.0:8080".into());
    info!("Starting the HTTP handler on {bind_to}");

    // This is to support hot reloading with catflap
    if let Some(fd) = std::env::var("LISTEN_FD")
        .ok()
        .and_then(|fd| fd.parse().ok())
    {
        app.listen(unsafe { TcpListener::from_raw_fd(fd) }).await?;
    } else {
        app.listen(bind_to).await?;
    }

    Ok(())
}
