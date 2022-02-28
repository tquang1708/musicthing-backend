use axum::{
    Router,
    routing::get_service,
    http::StatusCode
};
use std::net::SocketAddr;
use tower_http::{
    services::ServeDir,
    trace::TraceLayer
};

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
    // routes need full file name including extension
    // look into this later for removing suffix?
    // https://github.com/tokio-rs/axum/discussions/446
    let app = Router::new()
        .nest(
            "/",
            get_service(ServeDir::new("./static")) // file directory: absolute or relative to where `cargo r` is run
                .handle_error(|e: std::io::Error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", e),
                    )
                }),
        )
        .layer(TraceLayer::new_for_http());

    // app
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}