use std::{fmt::Debug, path::PathBuf};

use async_trait::async_trait;
use flate2::Compression;
use reqwest::Client;
use serde_json::Value as JsonValue;
use std::fs;
use ulid::Ulid;

use crate::Error;

pub struct SfsSpider {
    http_client: Client,
    output_path: PathBuf,
}

impl SfsSpider {
    pub fn new(options: SfsSpiderOptions) -> Self {
        println!("{:?}", options);
        let SfsSpiderOptions {
            user_agent: user_agent_opt,
            output_path,
        } = options;
        let user_agent = user_agent_opt.as_deref().unwrap_or(crate::APP_USER_AGENT);
        fs::create_dir_all(&output_path).expect("spiders/sfs: can't create output_path");
        let output_path = output_path
            .canonicalize()
            .expect("spiders/sfs: output_path error");
        tracing::warn!(user_agent, "configuring SfsSpider {:?}", output_path);
        let http_client = reqwest::Client::builder()
            .user_agent(user_agent)
            .brotli(true)
            .build()
            .expect("spiders/sfs: Building HTTP client");
        Self {
            http_client,
            output_path,
        }
    }
}

impl Debug for SfsSpider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SfsSpider {{ /* omitted */ }}")
    }
}

impl Default for SfsSpider {
    fn default() -> Self {
        Self::new(SfsSpiderOptions::default())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SfsSpiderOptions {
    pub user_agent: Option<String>,
    pub output_path: PathBuf,
}

impl Default for SfsSpiderOptions {
    fn default() -> Self {
        Self {
            user_agent: None,
            output_path: "./output".into(),
        }
    }
}
#[async_trait]
impl super::Spider for SfsSpider {
    type Item = (String, JsonValue);

    fn name(&self) -> String {
        String::from("sfs")
    }

    fn start_urls(&self) -> Vec<String> {
        let base_url = "https://data.riksdagen.se/dokumentlista/?sok=&doktyp=SFS&rm=&ts=&bet=&tempbet=&nr=&org=&iid=&avd=&webbtv=&talare=&exakt=&planering=&facets=&sort=rel&sortorder=desc&rapport=&utformat=json";
        let mut urls = Vec::new();

        for (from_year, to_year) in [
            (1880, 1900),
            (1901, 1920),
            (1921, 1940),
            (1941, 1960),
            (1961, 1980),
            (1981, 2000),
            (2001, 2020),
            (2021, 2023),
        ] {
            urls.push(format!(
                "{base_url}&from={from_year}-01-01&to={to_year}-12-31&a=s#soktraff"
            ))
        }
        urls
    }

    #[tracing::instrument]
    async fn scrape(&self, url: String) -> Result<(Vec<Self::Item>, Vec<String>), Error> {
        let mut new_urls = Vec::new();
        let mut items = Vec::new();

        let dokumentstatus_url = "https://data.riksdagen.se/dokumentstatus";
        let dokument_url = "https://data.riksdagen.se/dokument";
        tracing::debug!("calling {}", url);
        let response = self.http_client.get(&url).send().await.map_err(|err| {
            tracing::error!("Failed fetching: {:?}", err);
            err
        })?;

        tracing::trace!("response status: {}", response.status());

        if !response.status().is_success() {
            let status_code = response.status();
            tracing::error!(
                "The request returned '{}': '{}",
                response.status(),
                response.text().await?
            );
            return Err(Error::RequestReturnedError(status_code));
        }

        let item: JsonValue = response.json().await.map_err(|err| {
            tracing::error!("Failed parsing JSON: {}", err);
            err
        })?;
        if url.contains("dokumentlista") {
            if let Some(nasta_sida) = item["dokumentlista"].get("@nasta_sida") {
                if let Some(url) = nasta_sida.as_str() {
                    new_urls.push(url.into());
                }
            }
            for dokument in item["dokumentlista"]["dokument"]
                .as_array()
                .ok_or_else(|| {
                    Error::UnexpectedJsonFormat("'dokumentlist.dokument' is not an array".into())
                })?
            {
                let dok_id = dokument["dok_id"].as_str().ok_or_else(|| {
                    Error::UnexpectedJsonFormat("dokument is missing 'dok_id'".into())
                })?;
                // let new_url = if dok_id.contains("sfs-N") {
                //     format!("{dokument_url}/{dok_id}/json")
                // } else if dok_id.contains("riks") {
                //     format!("{dokumentstatus_url}/{dok_id}.json")
                // } else {
                //     format!("{dokumentstatus_url}/{dok_id}?utdata=json")
                // };
                let new_url = format!("{dokument_url}/{dok_id}.json");
                new_urls.push(new_url);
            }
        } else if url.contains("dokumentstatus") {
            tracing::trace!("scraping dokumentstatus");
        } else if url.contains("dokument") {
            tracing::trace!("scraping dokument");
            let mut create_new_url = true;
            if let Some(dokumentstatus) = item.get("dokumentstatus") {
                if let Some(dokument) = dokumentstatus.get("dokument") {
                    if let Some(_dokument) = dokument.get("dok_id") {
                        create_new_url = false;
                    }
                }
            }
            if create_new_url {
                let new_url = url.replace("dokument", "dokumentstatus");
                new_urls.push(new_url);
            }
        } else {
            tracing::warn!("don't know how to scrape '{}'", url);
        }
        items.push((url, item));
        Ok((items, new_urls))
    }

    #[tracing::instrument(skip(item))]
    async fn process(&self, item: Self::Item) -> Result<(), Error> {
        let (url, item) = item;
        let mut path = self.output_path.clone();
        let mut file_name = String::new();
        tracing::info!("analyzing url={}", url);
        if let Some(dokument_lista) = item.get("dokumentlista") {
            path.push("dokumentlista");
            file_name = dokument_lista["@q"]
                .as_str()
                .ok_or_else(|| {
                    tracing::error!("item={:?} url={}", item, url);
                    Error::UnexpectedJsonFormat("Can't find 'dokumentlista.@q".into())
                })?
                .replace('&', "_");
        } else if let Some(dokumentstatus) = item.get("dokumentstatus") {
            let dokument_typ = dokumentstatus["dokument"]["typ"]
                .as_str()
                .unwrap_or("NO_TYP");
            path.push(dokument_typ);
            let dokument_rm = dokumentstatus["dokument"]["rm"].as_str().unwrap_or("NO_RM");
            path.push(dokument_rm);

            file_name = dokumentstatus["dokument"]["dok_id"]
                .as_str()
                .unwrap_or_else(|| {
                    tracing::error!("no dok_id in item={:?} for url={}", item, url);
                    ""
                    // Error::UnexpectedJsonFormat("can't find 'dokument.dok_id'".into())
                })
                .replace(' ', "_")
                .replace('.', "_");
        }

        tokio::fs::create_dir_all(&path).await.map_err(|err| {
            tracing::error!("failed creating path='{}', url={}", path.display(), url);
            err
        })?;
        if file_name.is_empty() {
            path.push(
                format!("unknown-{}-{}", url, Ulid::new())
                    .replace('/', "_")
                    .replace(':', "")
                    .replace(' ', "_")
                    .replace('.', "_"),
            );
        } else {
            path.push(&file_name);
        }
        // let file_name = format!("{file_name}.json");
        path.set_extension("json.gz");
        let span = tracing::info_span!("writing output", "{}", path.display());
        let _enter = span.enter();
        tracing::debug!("creating file");
        let file = std::fs::File::create(path).map_err(|err| {
            tracing::error!("failed creating file, url={}", url);
            err
        })?;
        let compress_writer = flate2::write::GzEncoder::new(file, Compression::default());
        let writer = std::io::BufWriter::new(compress_writer);
        tracing::debug!("writing JSON");
        serde_json::to_writer(writer, &item).map_err(|err| {
            tracing::error!("failed writing JSON, url={}", url);
            err
        })?;
        Ok(())
    }
}
