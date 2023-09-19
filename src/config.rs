use std::str::FromStr;

use color_eyre::eyre::{eyre, Context};
use color_eyre::Result;
use serde::Deserialize;
use toml::Table;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub toffee: ToffeeConfig,
    pub modes: Option<Table>,
}

#[derive(Deserialize, Debug)]
pub struct ToffeeConfig {}

#[derive(Deserialize, Debug)]
pub struct BackendConfig {
    pub backend: String,
}

impl FromStr for Config {
    type Err = color_eyre::eyre::Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        toml::from_str(s).wrap_err("failed to parse config")
    }
}

impl Config {
    pub fn mode<C: for<'de> Deserialize<'de>>(&self, name: &str) -> Result<C> {
        let mode = self
            .modes
            .as_ref()
            .ok_or(eyre!("no modes configured"))?
            .get(name)
            .ok_or(eyre!("unknown mode {name}"))?;
        let mode_config =
            C::deserialize(mode.clone()).wrap_err("failed to deserialize config")?;

        Ok(mode_config)
    }
}
