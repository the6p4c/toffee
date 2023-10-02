mod emoji_label;

use eframe::egui;
use log::debug;
use serde::Deserialize;

use crate::backends::emoji::emoji_label::EmojiLabel;
use crate::backends::{Backend, Entries, NewBackend};

pub struct Emoji {
    svg_path: String,
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
pub struct Config {
    svg_path: String,
}

const EMOJI: &[u8] = include_bytes!("../../../emoji.txt");

impl NewBackend for Emoji {
    type Config = Config;

    fn new(_cc: &eframe::CreationContext<'_>, config: Self::Config) -> Self {
        let s = String::from_utf8(EMOJI.to_owned()).unwrap();
        let entries = s
            .lines()
            .map(|line| {
                let (codepoints, name) = line.split_once(';').unwrap();
                let emoji = codepoints
                    .split(' ')
                    .map(|codepoint| {
                        char::from_u32(u32::from_str_radix(codepoint, 16).unwrap()).unwrap()
                    })
                    .collect::<String>();
                Entry {
                    emoji,
                    name: name.to_owned(),
                }
            })
            .take(1000)
            .collect();

        Self {
            svg_path: config.svg_path,
            entries,
        }
    }
}

impl<'entry> Backend<'entry> for Emoji {
    type Entry = &'entry Entry;

    fn entries(&'entry self, query: &str) -> Entries<Self::Entry> {
        Entries {
            counter: None, // TODO: counter
            entries: self.entries.iter().collect(),
        }
    }

    fn entry_contents(&self, ui: &mut egui::Ui, entry: Self::Entry) {
        ui.horizontal(|ui| {
            egui::Frame::none().outer_margin(2.0).show(ui, |ui| {
                ui.add(EmojiLabel::new(&self.svg_path, &entry.emoji));
            });
            ui.label(&entry.name);
        });
    }

    fn on_selected(&self, entry: Self::Entry) {
        debug!("selected emoji {}", entry.emoji);
    }
}

pub struct Entry {
    emoji: String,
    name: String,
}
