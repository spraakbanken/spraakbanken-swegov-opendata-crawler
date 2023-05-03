use reqwest::Client;

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}

async fn try_main() -> anyhow::Result<()> {
    let client = client()?;
    let res = client.get("https://www.rust-lang.org").send().await?;

    println!("Status code: {}", res.status());
    Ok(())
}

// == Client ==
// Name your user agent after your app?
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

fn client() -> anyhow::Result<Client> {
    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;
    Ok(client)
}
