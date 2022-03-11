use std::{net::SocketAddr, time::Duration};
use axum::{
    http::Method,
    Router,
    routing::get,
    extract::{Extension}
};
use tower_http::cors::{CorsLayer, Origin};
use sqlx::postgres::PgPoolOptions;

use handlers::demo;
mod handlers;

#[tokio::main]
async fn main() {
    // set the RUST_LOG env for logging
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var(
            "RUST_LOG",
            "musicthing=debug,tower_http=debug",
        )
    };
    tracing_subscriber::fmt::init();

    // metadata db connection
    let db_connection_str = "postgres://postgres:password@localhost/musicthing-metadb".to_string();
    let pool = PgPoolOptions::new()
        .max_connections(5) // move to cfg
        .connect_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("Can't connect to database");

    // app routing
    let app = Router::new()
        .route("/api/test", get(demo::basic_handler))
        .route("/api/db", get(demo::connection_pool_extractor_handler))
        .layer(CorsLayer::new()
            .allow_origin(Origin::exact("http://localhost:3000".parse().unwrap()))
            .allow_methods(vec![Method::GET]))
        .layer(Extension(pool));

    // app
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
