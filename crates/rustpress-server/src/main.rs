use rustpress_server::{App, Config};
use tracing_subscriber::EnvFilter;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 * 1024 * 1024).build().expect("tokio runtime");
    runtime.block_on(async { runtime.spawn(run_server()).await.expect("server task") });
}

async fn run_server() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))).init();
    if let Err(message) = run().await {
        eprintln!("rustpress: {message}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let listener = tokio::net::TcpListener::bind(&config.listen).await.map_err(|e| format!("cannot listen on {}: {e}", config.listen))?;
    let app = App::new(config).await.map_err(|e| e.to_string())?;
    tracing::info!(address = %listener.local_addr().map_err(|e| e.to_string())?, "listening");
    axum::serve(listener, app.router().into_make_service_with_connect_info::<std::net::SocketAddr>()).with_graceful_shutdown(shutdown()).await.map_err(|e| e.to_string())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
