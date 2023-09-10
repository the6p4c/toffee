use eframe::egui;

mod modes;
mod toffee;

use modes::{DRun, Mode};

struct App<M: Mode> {
    mode: M,
    input: String,
}

impl<M: Mode> App<M> {
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

impl<M: Mode> eframe::App for App<M> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| self.mode.update(ui, &mut self.input));
    }
}

fn main() {
    env_logger::init();

    let mode = DRun::new();

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::emath::Vec2::new(500.0, 200.0)),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "toffee",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, mode))),
    )
    .expect("app run failed");
}
