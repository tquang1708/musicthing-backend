use axum::{
    http::Method,
    Router,
    routing::get,
};

use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Origin};

#[tokio::main]
async fn main() {
    // set the RUST_LOG env
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var(
            "RUST_LOG",
            "musicthing=debug,tower_http=debug",
        )
    };
    tracing_subscriber::fmt::init();

    // app routing
    let app = Router::new().route("/api/test", get(handler)).layer(
        CorsLayer::new()
            .allow_origin(Origin::exact("http://localhost:3000".parse().unwrap()))
            .allow_methods(vec![Method::GET]),
    );

    // app
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> &'static str {
    ":q!"
}
