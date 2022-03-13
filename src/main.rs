use std::{
    net::SocketAddr,
    time::Duration,
    error::Error,
};
use axum::{
    http::{Method, StatusCode, Uri},
    handler::Handler,
    response::{IntoResponse},
    Router,
    routing::get,
    extract::{Extension}
};
use tower_http::{
    cors::{CorsLayer, Origin},
    trace::TraceLayer,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt
};

mod handlers;
mod utils;

use crate::handlers::{demo, reload, list, play};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //set up tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // metadata db connection
    let config = utils::parse_cfg()?; // load config
    let db_connection_str = config.database_connection_str;
    let pool = PgPoolOptions::new()
        .max_connections(config.max_db_connections)
        .connect_timeout(Duration::from_secs(config.db_connection_timeout_seconds))
        .connect(&db_connection_str)
        .await
        .expect("Can't connect to database");

    // app routing
    let app = Router::new()
        .route("/api/test", get(demo::basic_handler))
        .route("/api/db", get(demo::connection_pool_extractor_handler))
        .route("/api/reload", get(reload::reload_handler))
        .route("/api/hard_reload", get(reload::hard_reload_handler))
        .route("/api/list", get(list::list_handler))
        .route("/api/play", get(play::play_handler))
        .layer(CorsLayer::new()
            .allow_origin(Origin::exact("http://localhost:3000".parse().unwrap()))
            .allow_methods(vec![Method::GET]))
        .layer(Extension(pool))
        .layer(TraceLayer::new_for_http());
    
    // 404 fallback
    let app = app.fallback(handler_404.into_service());

    // app
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn handler_404(uri: Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, format!("404 Not Found - No route for {}", uri))
}