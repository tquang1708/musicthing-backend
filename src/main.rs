use std::{net::SocketAddr, time::Duration};
use axum::{
    http::{Method, StatusCode},
    Router,
    routing::get,
    extract::{Extension}
};
use tower_http::cors::{CorsLayer, Origin};
use sqlx::postgres::{PgPool, PgPoolOptions};

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
        .route("/api/test", get(handler))
        .route("/api/db", get(connection_pool_extractor))
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

async fn handler() -> &'static str {
    ":q!"
}

async fn connection_pool_extractor(
    Extension(pool): Extension<PgPool>
) -> Result<String, (StatusCode, String)> {
    sqlx::query_scalar("SELECT * FROM track;")
        .fetch_one(&pool)
        .await
        .map_err(internal_error)
}

// Utility function for mapping errors into 500 http response
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
