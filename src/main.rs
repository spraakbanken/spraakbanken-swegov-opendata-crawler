use std::time::Duration;

use reqwest::Client;
use tracing::{event, Level};

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
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber)?;

    let crawler = Crawler::new(Duration::from_millis(500), 2, 50);
    let client = client()?;
    let res = client.get("https://www.rust-lang.org").send().await?;

    println!("Status code: {}", res.status());
    Ok(())
}

// == Client ==
// Name your user agent after your app?
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

fn client() -> anyhow::Result<Client> {
    event!(Level::WARN, user_agent = APP_USER_AGENT);
    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;
    Ok(client)
}
