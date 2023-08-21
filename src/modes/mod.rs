use eframe::egui;

mod drun;

pub use drun::DRun;

pub trait Mode {
    fn update(&mut self, ui: &mut egui::Ui, input: &mut String);
}
