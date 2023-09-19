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
pub struct ToffeeConfig {}

#[derive(Deserialize, Debug)]
pub struct ModeConfig<C = Table> {
    pub backend: String,
    #[serde(flatten)]
    pub backend_config: C,
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

impl ModeConfig {
    // TODO: rename me
    pub fn drill_down<C: for<'de> Deserialize<'de>>(self) -> Result<ModeConfig<C>> {
        Ok(ModeConfig {
            backend: self.backend,
            backend_config: C::deserialize(self.backend_config)?,
        })
    }
}
