use crate::gui::log::Message;
use crossbeam::channel;

pub struct LogPanel {
    receiver: channel::Receiver<Message>,
    messages: Vec<Message>,
    auto_scroll: bool,
}

impl LogPanel {
    pub fn new(receiver: channel::Receiver<Message>) -> Self {
        Self {
            receiver,
            messages: Vec::new(),
            auto_scroll: true,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        while let Ok(message) = self.receiver.try_recv() {
            self.messages.push(message);
        }

        ui.set_height(125.0);

        ui.horizontal(|ui| {
            ui.heading("Log");
            ui.separator();
            ui.checkbox(&mut self.auto_scroll, "Auto Scroll")
                .on_hover_text(
                "Automatically scroll to view new messages when the scroll bar is at the bottom.",
            );
        });

        ui.separator();

        let row_height = ui.text_style_height(&egui::TextStyle::Body);

        egui::ScrollArea::both()
            .stick_to_bottom(self.auto_scroll)
            .show_rows(ui, row_height, self.messages.len(), |ui, rows| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                let grid = egui::Grid::new("messages").num_columns(3);
                grid.show(ui, |ui| {
                    for message in rows.map(|i| &self.messages[i]) {
                        message.draw(ui);
                    }
                });
            });
    }
}
