mod backends;
mod config;
mod toffee;

use std::{env, fs};

use color_eyre::eyre::{bail, eyre, Context, Result};
use log::info;

use crate::backends::{Backend, NewBackend};
use crate::config::Config;
use crate::toffee::{Toffee, ToffeeData};

struct Mode<B: for<'entry> Backend<'entry>> {
    name: String,
    backend: B,
    query: String,
}

impl<B: for<'entry> Backend<'entry> + NewBackend + 'static> Mode<B> {
    fn start(config: Config, name: String) -> Result<()> {
        let (initial_width, initial_height) = config.toffee.initial_size.unwrap_or((500, 200));

        let native_options = eframe::NativeOptions {
            initial_window_size: Some(egui::emath::Vec2::new(
                initial_width as f32,
                initial_height as f32,
            )),
            ..eframe::NativeOptions::default()
        };

        eframe::run_native(
            "toffee",
            native_options,
            Box::new(|cc| Box::new(Self::new(cc, config, name))),
        )
        .map_err(|_| eyre!("app run_native failed"))
    }

    fn new(cc: &eframe::CreationContext<'_>, config: Config, name: String) -> Self {
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

        // start the backend
        // TODO: error handling
        let (_, mode_config) = config.split(&name).unwrap();

        Self {
            name,
            backend: B::new(mode_config.backend),
            query: String::new(),
        }
    }
}

impl<B: for<'entry> Backend<'entry>> eframe::App for Mode<B> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let entries = self.backend.entries(&self.query);

                let toffee_data = ToffeeData {
                    mode: &self.name,
                    counter: entries.counter.map(|c| (c.visible, c.total)),
                    entries: entries.entries,
                };

                let toffee = Toffee::new("toffee", toffee_data, &mut self.query)
                    .show(ui, |ui, entry| self.backend.entry_contents(ui, entry));

                if let Some(selected_entry) = toffee.selected_entry {
                    self.backend.on_selected(selected_entry);
                }
            });
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let mut args = env::args();
    args.next(); // skip binary name
    let mode = args.next().ok_or(eyre!("need a mode to run"))?;

    let config = fs::read_to_string("config.toml")
        .wrap_err("failed to read config file")?
        .parse::<Config>()?;
    let backend = config.backend(&mode)?;

    info!("launching mode {mode} with backend {backend}");
    match backend.as_str() {
        "drun" => Mode::<backends::DRun>::start(config, mode),
        _ => bail!("unknown backend {backend}"),
    }
}
