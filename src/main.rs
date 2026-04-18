mod config;
mod events;
mod forgejo;
mod gotify;
mod notification;
mod server;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::Settings, gotify::GotifyClient, server::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = match Settings::load() {
        Ok(settings) => settings,
        Err(err) => {
            init_logging("info")?;
            tracing::error!(%err, "invalid configuration");
            return Err(err);
        }
    };
    init_logging(&settings.logging.filter)?;

    let gotify = GotifyClient::new(&settings.gotify);
    server::serve(AppState::new(settings, gotify)).await
}

fn init_logging(filter: &str) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_new(filter)?)
        .with(tracing_subscriber::fmt::layer())
        .init();
    Ok(())
}
