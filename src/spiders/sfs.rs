use std::{fmt::Debug, path::PathBuf};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime};
use flate2::Compression;
use reqwest::{Client, ClientBuilder};
use serde_json::Value as JsonValue;
use std::{fs, io};
use tracing::{event, info_span, Level};
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
        let user_agent = user_agent_opt
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(crate::APP_USER_AGENT);
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
    type Item = JsonValue;

    fn name(&self) -> String {
        String::from("sfs")
    }

    fn start_urls(&self) -> Vec<String> {
        let base_url = "https://data.riksdagen.se/dokumentlista/?sok=&doktyp=SFS&utdata=json";
        let base_url = "https://data.riksdagen.se/dokumentlista/?sok=&doktyp=SFS&rm=&ts=&bet=&tempbet=&nr=&org=&iid=&avd=&webbtv=&talare=&exakt=&planering=&facets=&sort=rel&sortorder=desc&rapport=&utformat=json";
        // let base_url = "https://data.riksdagen.se/dokumentlista/?sok=&doktyp=SFS&rm=&from=1880-01-01&tom=1920-01-01&ts=&bet=&tempbet=&nr=&org=&iid=&avd=&webbtv=&talare=&exakt=&planering=&facets=&sort=rel&sortorder=desc&rapport=&utformat=json&a=s#soktraff";
        let mut urls = Vec::new();
        // vec![String::from(
        //     "https://data.riksdagen.se/dokumentlista/?sok=&doktyp=SFS&rm=&from=1880-01-01&tom=&ts=&bet=&tempbet=&nr=&org=&iid=&avd=&webbtv=&talare=&exakt=&planering=&facets=&sort=rel&sortorder=desc&rapport=&utformat=json&a=s#soktraff",
        //     "https://data.riksdagen.se/dokumentlista/?sok=&doktyp=SFS&rm=&from=1880-01-01&tom=&ts=&bet=&tempbet=&nr=&org=&iid=&avd=&webbtv=&talare=&exakt=&planering=&facets=&sort=rel&sortorder=desc&rapport=&utformat=json&a=s#soktraff",
        // )];
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

        let dokument_url = "https://data.riksdagen.se/dokumentstatus";
        tracing::debug!("calling {}", url);
        let response = self.http_client.get(&url).send().await.map_err(|err| {
            tracing::error!("Failed fetching: {:?}", err);
            err
        })?;

        tracing::trace!("response status: {}", response.status());

        if !response.status().is_success() {
            let status_code = response.status().clone();
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
                new_urls.push(format!("{dokument_url}/{}?utdata=json", dok_id))
            }
        } else if url.contains("dokumentstatus") {
            tracing::trace!("scraping dokumentstatus");
        } else {
            tracing::error!("don't know how to scrape '{}'", url);
        }
        items.push(item);
        Ok((items, new_urls))
    }

    #[tracing::instrument(skip(item))]
    async fn process(&self, item: Self::Item) -> Result<(), Error> {
        let mut path = self.output_path.clone();
        let mut file_name = String::new();
        tracing::trace!("{:?}", item);
        if let Some(dokument_lista) = item.get("dokumentlista") {
            path.push("dokumentlista");
            file_name = dokument_lista["@q"]
                .as_str()
                .ok_or_else(|| {
                    tracing::error!("item={:?}", item);
                    Error::UnexpectedJsonFormat("Can't find 'dokumentlista.@q".into())
                })?
                .replace("&", "_");
        } else if let Some(dokumentstatus) = item.get("dokumentstatus") {
            let dokument_typ = dokumentstatus["dokument"]["typ"]
                .as_str()
                .unwrap_or("NO_TYP");
            path.push(dokument_typ);
            let dokument_rm = dokumentstatus["dokument"]["rm"].as_str().unwrap_or("NO_RM");
            path.push(dokument_rm);

            let dok_id = dokumentstatus["dokument"]["dok_id"]
                .as_str()
                .ok_or_else(|| {
                    Error::UnexpectedJsonFormat("can't find 'dokument.dok_id'".into())
                })?;
            let file_name = dok_id.replace(" ", "_");
        }

        tracing::debug!("creating dirs {:?}", path);
        tokio::fs::create_dir_all(&path).await.map_err(|err| {
            tracing::error!("failed creating path='{}'", path.display());
            err
        })?;
        if file_name.is_empty() {
            path.push(format!("unknown-{}", Ulid::new()));
        } else {
            path.push(&file_name);
        }
        // let file_name = format!("{file_name}.json");
        path.set_extension("json.gz");
        let span = info_span!("creating file filename='{}'", "{}", path.display());
        let _enter = span.enter();
        tracing::debug!("creating file {:?}", path);
        let file = std::fs::File::create(path).map_err(|err| {
            tracing::error!("failed creating file");
            err
        })?;
        let compress_writer = flate2::write::GzEncoder::new(file, Compression::default());
        let writer = std::io::BufWriter::new(compress_writer);
        tracing::debug!("writing JSON");
        serde_json::to_writer(writer, &item).map_err(|err| {
            tracing::error!("failed writing JSON");
            err
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum JsonOrLista {
    Dokument(JsonValue),
    Lista(DokumentLista),
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DokumentStatusPage {
    dokumentstatus: DokumentStatus,
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DokumentStatus {
    dokument: SfsDokument,
    dokuppgift: DokUppgift,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SfsDokument {
    dok_id: String,
    rm: String,
    beteckning: String,
    typ: String,
    subtyp: String,
    organ: String,
    nummer: String,
    slutnummer: String,
    #[serde(with = "my_date_format")]
    datum: NaiveDateTime,
    #[serde(with = "my_date_format")]
    publicerad: NaiveDateTime,
    #[serde(with = "my_date_format")]
    systemdatum: NaiveDateTime,
    titel: String,
    text: String,
    html: String,
    dokumentnamn: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DokUppgift {
    uppgift: Vec<Uppgift>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Uppgift {
    kod: String,
    namn: String,
    text: String,
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DokumentListaPage {
    dokumentlista: DokumentLista,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DokumentLista {
    dokument: Vec<DokumentListaDokument>,
    #[serde(rename = "@nasta_sida")]
    nasta_sida: String,
    #[serde(rename = "@sida")]
    sida: String,
    #[serde(rename = "@q")]
    q: String,
    #[serde(rename = "@sidor")]
    sidor: String,
    #[serde(rename = "@traffar")]
    traffar: String,
    #[serde(rename = "@traff_fran")]
    traff_fran: String,
    #[serde(rename = "@traff_till")]
    traff_till: String,
    #[serde(rename = "@version")]
    version: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DokumentListaDokument {
    id: String,
    dok_id: String,
    traff: String,
    domain: String,
    database: String,
    datum: NaiveDate,
    #[serde(with = "my_date_format")]
    publicerad: NaiveDateTime,
    #[serde(with = "my_date_format")]
    systemdatum: NaiveDateTime,
}

mod my_date_format {
    use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}
