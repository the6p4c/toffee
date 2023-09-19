use crate::backends::{Backend, DRun, NewBackend};
use crate::config::Config;
use crate::toffee::{Toffee, ToffeeData};
use color_eyre::eyre::{eyre, Context};
use color_eyre::Result;
use eframe::egui;
use serde::Deserialize;
use std::{env, fs};

pub struct Mode<B: for<'entry> Backend<'entry>> {
    name: String,
    backend: B,
}

impl<B: for<'entry> Backend<'entry> + NewBackend> Mode<B> {
    pub fn new(name: impl Into<String>, config: Config) -> Result<Self> {
        let name = name.into();
        let mode_config = config.mode(&name)?;

        Ok(Self {
            name,
            backend: B::new(mode_config),
        })
    }
}

impl<B: for<'entry> Backend<'entry> + 'static> Mode<B> {
    pub fn run(self) -> Result<()> {
        let native_options = eframe::NativeOptions {
            initial_window_size: Some(egui::emath::Vec2::new(500.0, 200.0)),
            ..eframe::NativeOptions::default()
        };

        eframe::run_native(
            "toffee",
            native_options,
            Box::new(|cc| Box::new(App::from_mode(cc, self))),
        )
        .map_err(|_| eyre!("app run failed"))?;

        Ok(())
    }
}

struct App<B: for<'entry> Backend<'entry>> {
    name: String,
    backend: B,
    input: String,
}

impl<B: for<'entry> Backend<'entry>> App<B> {
    fn from_mode(cc: &eframe::CreationContext<'_>, mode: Mode<B>) -> Self {
        let ctx = &cc.egui_ctx;

        // scale the ui up a bit
        ctx.set_pixels_per_point(1.5);

        // add monofur, use as default proportional font
        // TODO: can we do this without explicit paths and using the system's fonts?
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "monofur".to_owned(),
            egui::FontData::from_static(include_bytes!("../monof55.ttf")),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "monofur".to_owned());
        ctx.set_fonts(fonts);

        Self {
            name: mode.name,
            backend: mode.backend,
            input: String::new(),
        }
    }
}

impl<M: for<'entry> Backend<'entry>> eframe::App for App<M> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let toffee_data = ToffeeData {
                    mode: "drun",
                    counter: None,
                    entries: self.backend.entries(&self.input),
                };
                let toffee = Toffee::new("toffee", toffee_data, &mut self.input)
                    .show(ui, |ui, entry| self.backend.entry_contents(ui, entry));
                if let Some(selected_entry) = toffee.selected_entry {
                    self.backend.on_selected(selected_entry);
                }
            });
    }
}
