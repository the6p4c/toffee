use std::str::FromStr;

use color_eyre::eyre::{eyre, Context, Report, Result};
use serde::Deserialize;
use toml::Table;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub toffee: ToffeeConfig,
    modes: Option<Table>,
}

#[derive(Deserialize, Debug)]
pub struct ToffeeConfig {
    pub initial_size: Option<(usize, usize)>,
}

impl FromStr for Config {
    type Err = Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        toml::from_str(s).wrap_err("failed to parse config")
    }
}

impl Config {
    fn mode(&self, name: &str) -> Result<ModeConfig> {
        let modes = self.modes.as_ref().ok_or(eyre!("no modes configured"))?;
        let mode = modes.get(name).ok_or(eyre!("unknown mode {name}"))?;
        let mode_config =
            ModeConfig::deserialize(mode.clone()).wrap_err("failed to deserialize config")?;

        Ok(mode_config)
    }

    pub fn backend(&self, name: &str) -> Result<String> {
        Ok(self.mode(name)?.meta.backend)
    }

    pub fn split<M: for<'de> Deserialize<'de>>(
        self,
        name: &str,
    ) -> Result<(ToffeeConfig, ModeConfig<M>)> {
        let mode = self.mode(name)?;
        let mode = ModeConfig {
            meta: mode.meta,
            backend: M::deserialize(mode.backend)?,
        };

        Ok((self.toffee, mode))
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
