use color_eyre::eyre::Context;
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
pub struct ModeConfig {
    pub backend: String,
}

impl Config {
    pub fn from_str(s: &str) -> Result<Self> {
        toml::from_str(s).wrap_err("failed to parse config")
    }
}
