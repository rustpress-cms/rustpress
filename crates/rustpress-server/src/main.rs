use rustpress_server::{App, Config};
use tracing_subscriber::EnvFilter;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 * 1024 * 1024).build().expect("tokio runtime");
    let task = if rustpress_server::cgi::is_cgi() { runtime.spawn(run_cgi()) } else { runtime.spawn(run_server()) };
    runtime.block_on(async { task.await.expect("server task") });
}

/// One request from the web server (CGI); log lines go to its error log.
async fn run_cgi() {
    tracing_subscriber::fmt().with_writer(std::io::stderr).with_ansi(false).with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))).init();
    let started = std::time::Instant::now();
    let result = async {
        let config = rustpress_server::cgi::config().map_err(|e| e.to_string())?;
        let app = App::for_one_request(config).await.map_err(|e| e.to_string())?;
        let loaded = started.elapsed();
        rustpress_server::cgi::serve(app).await.map_err(|e| e.to_string())?;
        tracing::debug!(load_ms = loaded.as_secs_f64() * 1000.0, total_ms = started.elapsed().as_secs_f64() * 1000.0, "cgi request");
        Ok::<_, String>(())
    }
    .await;
    if let Err(message) = result {
        eprintln!("rustpress: {message}");
        rustpress_server::cgi::write_failure();
        std::process::exit(1);
    }
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
