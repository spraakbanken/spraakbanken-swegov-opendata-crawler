use async_trait::async_trait;
use reqwest::{Client, ClientBuilder};
use tracing::{event, Level};

use crate::error::Error;

pub struct SfsSpider {
    http_client: Client,
}

impl SfsSpider {
    pub fn new() -> Self {
        event!(Level::WARN, user_agent = crate::APP_USER_AGENT);
        let http_client = reqwest::Client::builder()
            .user_agent(crate::APP_USER_AGENT)
            .brotli(true)
            .build()
            .expect("spiders/sfs: Building HTTP client");
        Self { http_client }
    }
}

#[async_trait]
impl super::Spider for SfsSpider {
    type Item = ();

    fn name(&self) -> String {
        String::from("sfs")
    }

    fn start_urls(&self) -> Vec<String> {
        vec![String::from(
            "https://data.riksdagen.se/dokumentlista/?sok=&doktyp=SFS&rm=&from=&tom=&ts=&bet=&tempbet=&nr=&org=&iid=&avd=&webbtv=&talare=&exakt=&planering=&facets=&sort=rel&sortorder=desc&rapport=&utformat=json&a=s#soktraff",
        )]
    }

    async fn scrape(&self, url: String) -> Result<(Vec<Self::Item>, Vec<String>), Error> {
        event!(Level::DEBUG, "calling {}", url);
        let result = self.http_client.get(&url).send().await?;
        event!(Level::TRACE, "response status: {}", result.status());
        let dokument_lista: Root = result.json().await?;
        println!("lista={:?}", dokument_lista);
        todo!("impl scrape")
    }

    async fn process(&self, item: Self::Item) -> Result<(), Error> {
        todo!("impl process")
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Root {
    dokumentlista: DokumentLista,
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DokumentLista {
    dokument: Vec<Dokument>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Dokument {
    id: String,
    dok_id: String,
}
