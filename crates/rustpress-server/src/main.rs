use rustpress_server::install::{Gate, Installer};
use rustpress_server::{App, Config};
use tracing_subscriber::EnvFilter;

fn main() {
    let cgi = rustpress_server::cgi::is_cgi();
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    if cgi {
        // One request: a thread per core of a large shared host only costs start-up time.
        builder.worker_threads(2);
    }
    let runtime = builder.enable_all().thread_stack_size(16 * 1024 * 1024).build().expect("tokio runtime");
    let task = if cgi { runtime.spawn(run_cgi()) } else { runtime.spawn(run_server()) };
    runtime.block_on(async { task.await.expect("server task") });
}

/// One request from the web server (CGI); log lines go to its error log.
async fn run_cgi() {
    tracing_subscriber::fmt().with_writer(std::io::stderr).with_ansi(false).with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))).init();
    let started = std::time::Instant::now();
    let result = async {
        let config = rustpress_server::cgi::config().map_err(|e| e.to_string())?;
        rustpress_db::set_one_request(true);
        let router = if config.database_url.is_empty() {
            match Installer::needed(&config).await {
                Some(installer) => installer.router(),
                None => return Err(rustpress_server::Config::load_with(None, Some(config.wp_root.clone())).err().map_or_else(|| "no database configured".to_owned(), |e| e.to_string())),
            }
        } else {
            match App::for_one_request(config.clone()).await {
                Ok(app) => app.router(),
                // A database without a site (yet): the installer.
                Err(e) => match Installer::needed(&config).await {
                    Some(installer) => installer.router(),
                    None => return Err(e.to_string()),
                },
            }
        };
        let loaded = started.elapsed();
        rustpress_server::cgi::serve(router, started, loaded).await.map_err(|e| e.to_string())?;
        tracing::debug!(load_ms = loaded.as_secs_f64() * 1000.0, total_ms = started.elapsed().as_secs_f64() * 1000.0, "cgi request");
        Ok::<_, String>(())
    }
    .await;
    if let Err(message) = result {
        eprintln!("rustpress: {message}");
        rustpress_server::cgi::write_failure(&message);
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
    let load = || Config::load_without_database(std::env::var_os("RUSTPRESS_CONFIG").map(std::path::PathBuf::from), None);
    let listen = load().map_err(|e| e.to_string())?.listen;
    let listener = tokio::net::TcpListener::bind(&listen).await.map_err(|e| format!("cannot listen on {listen}: {e}"))?;
    let gate = Gate::start(Box::new(load)).await?;
    tracing::info!(address = %listener.local_addr().map_err(|e| e.to_string())?, "listening");
    axum::serve(listener, gate.router().into_make_service_with_connect_info::<std::net::SocketAddr>()).with_graceful_shutdown(shutdown()).await.map_err(|e| e.to_string())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
