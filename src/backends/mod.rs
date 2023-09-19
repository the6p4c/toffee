mod drun;

use eframe::egui;
use serde::Deserialize;

pub use drun::DRun;

pub trait NewBackend {
    type Config: for<'de> Deserialize<'de>;

    fn new(config: Self::Config) -> Self;
}

pub trait Backend<'entry> {
    type Entry: Copy;

    fn entries(&'entry self, query: &str) -> Vec<Self::Entry>;
    fn entry_contents(&self, ui: &mut egui::Ui, entry: Self::Entry);

    fn on_selected(&self, entry: Self::Entry);
}
