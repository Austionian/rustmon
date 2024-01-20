use anyhow::{Context, Result};
use rustmon::startup;
use tracing::Level;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{filter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    // initialize tracing
    let filter = filter::Targets::new()
        .with_target("tower_http::trace::on_request", Level::TRACE)
        .with_target("tower_http::trace::on_response", Level::TRACE)
        .with_target("tower_http::trace::make_span", Level::DEBUG)
        .with_default(Level::INFO);

    let tracing_layer = fmt::layer();

    tracing_subscriber::registry()
        .with(tracing_layer)
        .with(filter)
        .init();

    let app = startup().await.context("Failed to start the server")?;

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .context("Failed to bind to port 3000.")?;

    println!("listening on 3000");
    axum::serve(listener, app)
        .await
        .context("Failed to start the app.")?;

    Ok(())
}
