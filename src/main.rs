use std::{
    net::SocketAddr,
    time::Duration,
    str::FromStr,
    borrow::Cow,
};
use axum::{
    Router,
    handler::Handler,
    http::{Method, StatusCode, Uri},
    response::{IntoResponse},
    routing::{get, get_service},
    extract::{Extension},
    error_handling::HandleErrorLayer,
};
use tower::{ServiceBuilder, BoxError};
use tower_http::{
    cors::{CorsLayer, Origin},
    trace::TraceLayer,
    services::ServeDir,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt
};
use anyhow::{Context, Result};

mod handlers;
mod utils;

use crate::{
    handlers::{demo, reload, list, play},
    utils::{SharedState, parse_cfg},
};

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    //set up tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // metadata db connection
    let config = parse_cfg()?; // load config
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
        .layer(Extension(pool))
        .layer(
            ServiceBuilder::new()
            // handle errors from middleware
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(config.max_state_concurrency_limit)
                .timeout(Duration::from_secs(config.state_timeout_seconds))
                .layer(TraceLayer::new_for_http())
                .layer(Extension(SharedState::default()))
                .into_inner(),
        )
        .nest(
            "/static",
            get_service(ServeDir::new(config.music_directory))
            .handle_error(|e: std::io::Error| async move {(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {}", e),
            )}),
        )
        .layer(CorsLayer::new()
            .allow_origin(Origin::exact(config.frontend_url.as_str().parse()?))
            .allow_methods(vec![Method::GET]))
        .layer(TraceLayer::new_for_http());
    
    // 404 fallback
    let app = app.fallback(handle_404.into_service());

    // app
    let addr = SocketAddr::from_str(config.backend_socket_addr.as_str())
        .context("Failed to parse backend_addr in config.json into a valid SocketAddr")?;
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn handle_404(uri: Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, format!("404 Not Found - No route for {}", uri))
}

// from axum's kv store example
// https://github.com/tokio-rs/axum/blob/main/examples/key-value-store/src/main.rs#L54
// async fn handle_error(e: BoxError) -> impl IntoResponse {
//     if e.is::<tower::timeout::error::Elapsed>() {
//         return (
//             StatusCode::REQUEST_TIMEOUT, 
//             Cow::from(format!("request time out. Error: {}", e)));
//     };

//     if e.is::<tower::load_shed::error::Overloaded>() {
//         return (
//             StatusCode::SERVICE_UNAVAILABLE, 
//             Cow::from(format!("service is overloaded. Error: {}", e)));
//     };

//     (
//         StatusCode::INTERNAL_SERVER_ERROR,
//         Cow::from(format!("Internal error: {}", e)),
//     )
// }