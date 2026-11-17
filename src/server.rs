use crate::{METRICS_SERVER, MetricsServer, REGISTRY};
use log::{error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub fn start_metrics_server(addr: String) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_for_thread = Arc::clone(&shutdown);

    let handle = thread::Builder::new()
        .name("prometheus_server".to_string())
        .spawn(move || {
            let server = match tiny_http::Server::http(&addr) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to start metrics server: {}", e);
                    return;
                }
            };

            info!("Prometheus metrics server running on {}/metrics", addr);

            loop {
                if shutdown_for_thread.load(Ordering::SeqCst) {
                    info!("Server shutting down gracefully");
                    break;
                }

                match server.recv_timeout(Duration::from_millis(200)) {
                    Ok(Some(request)) => {
                        if request.url() == "/metrics" {
                            use prometheus::Encoder;
                            let encoder = prometheus::TextEncoder::new();
                            let metric_families = REGISTRY.gather();
                            let mut buffer = vec![];
                            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                                error!("Failed to encode metrics: {}", e);
                                continue;
                            }

                            let response = tiny_http::Response::from_data(buffer);
                            let _ = request.respond(response);
                        } else {
                            let _ = request.respond(tiny_http::Response::empty(404));
                        }
                    }
                    Ok(None) => {
                        // timeout, loop again
                    }
                    Err(e) => {
                        error!("{e}");
                        break;
                    }
                }
            }
        });

    let server = MetricsServer {
        thread: handle.unwrap(),
        shutdown,
    };

    *METRICS_SERVER.lock().unwrap() = Some(server);
}

pub fn stop_metrics_server() {
    info!("Server shutdown requested");

    if let Some(server) = METRICS_SERVER.lock().unwrap().take() {
        server.shutdown.store(true, Ordering::Release);
        server.thread.join().unwrap();
    }
}
