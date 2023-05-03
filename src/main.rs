use std::{sync::Arc, time::Duration};

use reqwest::Client;
use tracing::{event, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::crawler::Crawler;

mod crawler;
mod error;
mod spiders;

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}

async fn try_main() -> anyhow::Result<()> {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("fetch_sfs=trace,warn"))
                .expect("telemetry: Creating EnvFilter"),
        )
        .finish();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber)?;

    let crawler = Crawler::new(Duration::from_millis(500), 2, 50);
    let spider = Arc::new(spiders::sfs::SfsSpider::new());
    crawler.run(spider).await;

    Ok(())
}

// == Client ==
// Name your user agent after your app?
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
