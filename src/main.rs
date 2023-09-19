mod backends;
mod config;
mod mode;
mod toffee;

use backends::DRun;
use color_eyre::eyre::{eyre, Context};
use color_eyre::Result;
use log::info;
use std::{env, fs};

use crate::config::{Config, BackendConfig};
use crate::mode::Mode;

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let mut args = env::args();
    args.next(); // skip binary name
    let mode = args.next().ok_or(eyre!("need a mode to run"))?;

    let config = fs::read_to_string("config.toml").wrap_err("failed to read config file")?;
    let config = config.parse::<Config>()?;

    let mode_config = config.mode::<BackendConfig>(&mode)?;
    let backend = mode_config.backend.as_str();

    info!("launching mode {mode} with backend {backend}");

    match backend {
        "drun" => Mode::<DRun>::new(mode, config)?.run()?,
        _ => Err(eyre!("unknown backend {backend} in mode {mode}"))?,
    }

    Ok(())
}
