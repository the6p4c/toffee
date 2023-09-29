use std::str::FromStr;

use color_eyre::eyre::{eyre, Context, Report, Result};
use serde::Deserialize;
use toml::Table;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub toffee: ToffeeConfig,
    pub modes: Option<Table>,
}

#[derive(Deserialize, Debug)]
pub struct ToffeeConfig {
    initial_size: Option<(usize, usize)>,
}

impl FromStr for Config {
    type Err = Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        toml::from_str(s).wrap_err("failed to parse config")
    }
}

impl Config {
    pub fn mode(&self, name: &str) -> Result<ModeConfig> {
        let modes = self.modes.as_ref().ok_or(eyre!("no modes configured"))?;
        let mode = modes.get(name).ok_or(eyre!("unknown mode {name}"))?;
        let mode_config =
            ModeConfig::deserialize(mode.clone()).wrap_err("failed to deserialize config")?;

        Ok(mode_config)
    }
}

#[derive(Deserialize, Debug)]
pub struct ModeConfig<C = Table> {
    pub meta: MetaConfig,
    #[serde(flatten)]
    pub backend: C,
}

#[derive(Deserialize, Debug)]
pub struct MetaConfig {
    pub name: Option<String>,
    pub backend: String,
}

impl ModeConfig {
    // TODO: rename me
    pub fn drill_down<C: for<'de> Deserialize<'de>>(self) -> Result<ModeConfig<C>> {
        Ok(ModeConfig {
            meta: self.meta,
            backend: C::deserialize(self.backend)?,
        })
    }
}
