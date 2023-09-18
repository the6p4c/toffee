mod config;
mod modes;
mod toffee;

use color_eyre::eyre::{eyre, Context};
use color_eyre::Result;
use eframe::egui;
use modes::{DRun, Mode, NewMode};
use serde::Deserialize;
use std::{env, fs};
use toffee::{Toffee, ToffeeData};

use crate::config::{Config, ModeConfig};

struct App<M: for<'entry> Mode<'entry>> {
    mode: M,
    input: String,
}

impl<M: for<'entry> Mode<'entry>> App<M> {
    fn new(cc: &eframe::CreationContext<'_>, mode: M) -> Self {
        let ctx = &cc.egui_ctx;

        // scale the ui up a bit
        ctx.set_pixels_per_point(1.5);

        // add monofur, use as default proportional font
        // TODO: can we do this without explicit paths and using the system's fonts?
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "monofur".to_owned(),
            egui::FontData::from_static(include_bytes!("/usr/share/fonts/TTF/monof55.ttf")),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "monofur".to_owned());
        ctx.set_fonts(fonts);

        Self {
            mode,
            input: String::new(),
        }
    }
}

impl<M: for<'entry> Mode<'entry>> eframe::App for App<M> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let toffee_data = ToffeeData {
                    mode: "drun",
                    counter: None,
                    entries: self.mode.entries(&self.input),
                };
                let toffee = Toffee::new("toffee", toffee_data, &mut self.input)
                    .show(ui, |ui, entry| self.mode.entry_contents(ui, entry));
                if let Some(selected_entry) = toffee.selected_entry {
                    self.mode.on_selected(selected_entry);
                }
            });
    }
}

fn run<M: for<'entry> Mode<'entry> + 'static>(backend: M) -> Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::emath::Vec2::new(500.0, 200.0)),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "toffee",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, backend))),
    )
    .map_err(|_| eyre!("app run failed"))?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let mut args = env::args();
    args.next();
    let selected_mode = args.next().ok_or(eyre!("need a mode to run"))?;

    let config = fs::read_to_string("config.toml").wrap_err("failed to read config file")?;
    let config = Config::from_str(&config)?;

    let modes = config.modes.ok_or(eyre!("no modes configured"))?;

    let mode = modes
        .get(&selected_mode)
        .ok_or(eyre!("unknown mode {selected_mode}"))?;
    let mode_common =
        ModeConfig::deserialize(mode.clone()).wrap_err("invalid common mode config")?;

    match mode_common.backend.as_str() {
        "drun" => {
            let config = <DRun as NewMode>::Config::deserialize(mode.clone())
                .wrap_err("invalid drun config")?;
            run(DRun::new(config))
        }
        _ => Err(eyre!("unknown backend {}", mode_common.backend)),
    }?;

    Ok(())
}
