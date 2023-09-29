mod backends;
mod config;
mod mode;
mod toffee;

use std::{env, fs};

use color_eyre::eyre::{bail, eyre, Context, Result};
use log::info;

use crate::config::Config;
use crate::mode::Mode;

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let mut args = env::args();
    args.next(); // skip binary name
    let mode = args.next().ok_or(eyre!("need a mode to run"))?;

    let config = fs::read_to_string("config.toml").wrap_err("failed to read config file")?;
    let config = config.parse::<Config>()?;

    let mode_config = config.mode(&mode)?;
    let backend = mode_config.meta.backend.as_str();

    info!("launching mode {mode} with backend {backend}");

    match backend {
        "drun" => Mode::<backends::DRun>::new(mode, config.toffee, mode_config)?.run()?,
        _ => bail!("unknown backend {backend}"),
    }

    Ok(())
}
