use std::path::PathBuf;

use crate::spiders::sfs::SfsSpiderOptions;

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub sfs: SfsSpiderOptions,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path =
        std::env::current_dir().expect("configuration: Failed to determine the current directory");

    let settings = config::Config::builder()
        .set_default("sfs.output_path", "./output")?
        .add_source(config::File::from(base_path.join("config.json")).required(false))
        .build()?;

    settings.try_deserialize::<Settings>()
}
