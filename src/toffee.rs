use eframe::egui;

pub struct ToffeeOutput<Entry: Copy> {
    pub input_changed: bool,
    pub selected_entry: Option<Entry>,
}

impl<Entry: Copy> ToffeeOutput<Entry> {
    pub fn input_changed(&self) -> bool {
        self.input_changed
    }

    pub fn selected_entry(&self) -> Option<Entry> {
        self.selected_entry
    }
}

pub struct ToffeeData<'data, Entry: Copy> {
    pub mode: &'data str,
    pub counter: Option<(usize, usize)>,
    pub entries: Vec<Entry>,
}

pub struct Toffee<'data, 'input, Entry: Copy> {
    id: egui::Id,
    data: ToffeeData<'data, Entry>,
    input: &'input mut dyn egui::TextBuffer,
}

impl<'data, 'input, Entry: Copy> Toffee<'data, 'input, Entry> {
    pub fn new(
        id: impl Into<egui::Id>,
        data: ToffeeData<'data, Entry>,
        input: &'input mut dyn egui::TextBuffer,
    ) -> Self {
        Self {
            id: id.into(),
            data,
            input,
        }
    }

    fn selected_index(&self, ui: &egui::Ui) -> usize {
        ui.memory(|m| m.data.get_temp(self.id.with("selected_index")))
            .unwrap_or_default()
    }

    fn set_selected_index(&self, ui: &mut egui::Ui, selected_index: usize) {
        ui.memory_mut(|m| {
            m.data
                .insert_temp(self.id.with("selected_index"), selected_index)
        });
    }

    fn update_selected_index(&mut self, ui: &mut egui::Ui) -> (usize, bool) {
        let initial_selected_index = self.selected_index(ui);
        let selected_index = initial_selected_index;

        let entries_len = self.data.entries.len();

        // jump to the top of the list if the selected index is out of bounds (e.g. after an entries
        // update that makes the list shorter)
        let selected_index = if selected_index < entries_len {
            selected_index
        } else {
            0
        };

        // handle arrow key up/down, without going out of bounds
        #[derive(PartialEq)]
        enum Motion {
            Up,
            Down,
        }

        let motion = ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::default(), egui::Key::ArrowUp) {
                Some(Motion::Up)
            } else if i.consume_key(egui::Modifiers::default(), egui::Key::ArrowDown) {
                Some(Motion::Down)
            } else {
                None
            }
        });
        let selected_index = match motion {
            // move up one entry
            Some(Motion::Up) if selected_index != 0 => selected_index - 1,
            // move down one entry (add one to the index rather than subtracting one from the length
            // to avoid an underflow when there are no entries)
            Some(Motion::Down) if selected_index + 1 != entries_len => selected_index + 1,
            // don't move - no motion, or it would put us out of bounds
            _ => selected_index,
        };

        // update state and pass selected index back to caller
        let selected_index_changed = selected_index != initial_selected_index;
        if selected_index_changed {
            self.set_selected_index(ui, selected_index);
        }

        (selected_index, selected_index_changed)
    }

    pub fn show(
        mut self,
        ui: &mut egui::Ui,
        entry_contents: impl Fn(&mut egui::Ui, Entry),
    ) -> ToffeeOutput<Entry> {
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

            let query = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(self.input).frame(false),
            );
            query.request_focus();
            query
        };

        let query = egui::TopBottomPanel::top(self.id.with("query"))
            .frame(egui::Frame::none())
            .show_inside(ui, query)
            .inner;

        let entries = |ui: &mut egui::Ui| {
            ui.set_min_width(ui.max_rect().width());
            ui.vertical(|ui| {
                let mut double_clicked = false;
                for (index, entry) in self.data.entries.iter().enumerate() {
                    let container = EntryContainer::from_selected_index(index, selected_index)
                        .show(ui, |ui| {
                            entry_contents(ui, *entry);
                        });

                    if selected_index_changed && selected_index == index {
                        container.response.scroll_to_me(None);
                    }
                    if container.response.clicked() {
                        self.set_selected_index(ui, index);
                    }
                    double_clicked |= container.response.double_clicked();
                }
                double_clicked
            })
            .inner
        };

        let entry_double_clicked = egui::CentralPanel::default()
            //.frame(egui::Frame::none()) // TODO: we want this, but it causes an overlap
            .show_inside(ui, |ui| {
                // remove vertical gaps between each result
                ui.style_mut().spacing.item_spacing.y = 0.0;

                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, entries)
                    .inner
            })
            .inner;

        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let selected_index_valid = selected_index < self.data.entries.len();
        let selected_entry = if (enter_pressed || entry_double_clicked) && selected_index_valid {
            Some(self.data.entries[selected_index])
        } else {
            None
        };

        ToffeeOutput {
            input_changed: query.changed(),
            selected_entry,
        }
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

    fn show<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let fill = match self.fill_style {
            EntryContainerFillStyle::Selected => egui::Color32::from_rgb(0x10, 0x42, 0x59),
            EntryContainerFillStyle::Even => egui::Color32::from_gray(27),
            EntryContainerFillStyle::Odd => egui::Color32::from_gray(35),
        };

        let frame = egui::Frame::none()
            .inner_margin(1.0)
            .fill(fill)
            .show(ui, |ui| {
                ui.set_min_width(ui.max_rect().width());
                add_contents(ui)
            });

        egui::InnerResponse {
            inner: frame.inner,
            response: frame.response.interact(egui::Sense::click()),
        }
    }
}
