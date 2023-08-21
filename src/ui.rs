use eframe::egui;

pub struct ToffeeData<'a, Entry> {
    pub mode: &'a str,
    pub counter: Option<(usize, usize)>,
    pub entries: &'a [&'a Entry],
}

pub struct Toffee<'a, Entry, EntryWidget>
where
    EntryWidget: Fn(&mut egui::Ui, &Entry) -> egui::Response,
{
    id: egui::Id,
    data: ToffeeData<'a, Entry>,
    input: &'a mut dyn egui::TextBuffer,
    entry_widget: EntryWidget,
}

impl<'a, Entry, EntryWidget> Toffee<'a, Entry, EntryWidget>
where
    EntryWidget: Fn(&mut egui::Ui, &Entry) -> egui::Response,
{
    pub fn new(
        id: impl Into<egui::Id>,
        data: ToffeeData<'a, Entry>,
        input: &'a mut dyn egui::TextBuffer,
        entry_widget: EntryWidget,
    ) -> Self {
        Self {
            id: id.into(),
            data,
            input,
            entry_widget,
        }
    }

    fn update_selected_index(&mut self, ui: &mut egui::Ui) -> (usize, bool) {
        #[derive(PartialEq)]
        enum Delta {
            Up,
            Down,
        }

        let delta = ui.input(|i| {
            if i.key_pressed(egui::Key::ArrowUp) {
                Some(Delta::Up)
            } else if i.key_pressed(egui::Key::ArrowDown) {
                Some(Delta::Down)
            } else {
                None
            }
        });

        // TODO: behaviour when searching (i.e. entries length changing)
        // - feels like we should keep the cursor on the current entry (not index, but the entry
        //   itself)
        // - search too deep then come back - don't move even though the list shrunk
        let selected_index_id = self.id.with("selected_index");
        let selected_index: usize = ui
            .memory(|m| m.data.get_temp(selected_index_id))
            .unwrap_or_default();
        if let Some(delta) = delta {
            let entries_len = self.data.entries.len();
            let selected_index = if selected_index >= entries_len {
                // we're already out of bounds
                0
            } else if delta == Delta::Up && selected_index != 0 {
                // move up one entry
                selected_index - 1
            } else if delta == Delta::Down && selected_index != entries_len - 1 {
                // move down one entry
                selected_index + 1
            } else {
                // don't move - it would put us out of bounds
                selected_index
            };

            ui.memory_mut(|m| m.data.insert_temp(selected_index_id, selected_index));

            (selected_index, true)
        } else {
            (selected_index, false)
        }
    }
}

impl<'a, Entry, EntryWidget> egui::Widget for Toffee<'a, Entry, EntryWidget>
where
    EntryWidget: Fn(&mut egui::Ui, &Entry) -> egui::Response,
{
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let (selected_index, selected_index_changed) = self.update_selected_index(ui);

        let query = |ui: &mut egui::Ui| {
            egui::SidePanel::left(self.id.with("query_mode"))
                .min_width(0.0)
                .resizable(false)
                .show_inside(ui, |ui| {
                    ui.add(egui::Label::new(self.data.mode).wrap(false));
                });

            if let Some(counter) = self.data.counter {
                egui::SidePanel::right(self.id.with("query_counter"))
                    .min_width(0.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        ui.add(
                            egui::Label::new(format!("{}/{}", counter.0, counter.1)).wrap(false),
                        );
                    });
            }

            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(self.input).frame(false),
            )
        };

        let entries = |ui: &mut egui::Ui| {
            ui.set_min_width(ui.max_rect().width());
            ui.vertical(|ui| {
                for (index, entry) in self.data.entries.iter().enumerate() {
                    EntryContainer::from_selected_index(index, selected_index).show(ui, |ui| {
                        let widget = (self.entry_widget)(ui, entry);
                        if selected_index_changed && selected_index == index {
                            widget.scroll_to_me(None);
                        }
                    });
                }
            });
        };

        egui::TopBottomPanel::top(self.id.with("query"))
            .frame(egui::Frame::none())
            .show_inside(ui, query);
        egui::CentralPanel::default()
            //.frame(egui::Frame::none()) // TODO: we want this, but it causes an overlap
            .show_inside(ui, |ui| {
                // remove vertical gaps between each result
                ui.style_mut().spacing.item_spacing.y = 0.0;

                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, entries);
            });

        ui.label("") // HACK: for a random response
    }
}

enum EntryContainerFillStyle {
    Selected,
    Even,
    Odd,
}

impl EntryContainerFillStyle {
    fn from_selected_index(index: usize, selected_index: usize) -> Self {
        if index == selected_index {
            EntryContainerFillStyle::Selected
        } else if index % 2 == 0 {
            EntryContainerFillStyle::Even
        } else {
            EntryContainerFillStyle::Odd
        }
    }
}

struct EntryContainer {
    fill_style: EntryContainerFillStyle,
}

impl EntryContainer {
    fn new(fill_style: EntryContainerFillStyle) -> Self {
        Self { fill_style }
    }

    fn from_selected_index(index: usize, selected_index: usize) -> Self {
        let fill_style = EntryContainerFillStyle::from_selected_index(index, selected_index);
        Self::new(fill_style)
    }

    fn show<R>(self, ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let fill = match self.fill_style {
            EntryContainerFillStyle::Selected => egui::Color32::from_rgb(0x10, 0x42, 0x59),
            EntryContainerFillStyle::Even => egui::Color32::from_gray(27),
            EntryContainerFillStyle::Odd => egui::Color32::from_gray(35),
        };

        egui::Frame::none()
            .inner_margin(1.0)
            .fill(fill)
            .show(ui, |ui| {
                ui.set_min_width(ui.max_rect().width());
                add_contents(ui)
            })
            .inner
    }
}
