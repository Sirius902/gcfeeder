use crate::gui::log::Message;
use crossbeam::channel;
use egui::Align;

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
        let mut received_message = false;

        while let Ok(message) = self.receiver.try_recv() {
            self.messages.push(message);
            received_message = true
        }

        ui.set_height(125.0);

        ui.horizontal(|ui| {
            ui.heading("Log");
            ui.separator();
            ui.checkbox(&mut self.auto_scroll, "Auto Scroll");
        });

        ui.separator();

        let row_height = ui.text_style_height(&egui::TextStyle::Body);

        egui::ScrollArea::both().show_rows(ui, row_height, self.messages.len(), |ui, rows| {
            let grid = egui::Grid::new("messages").num_columns(3);
            grid.show(ui, |ui| {
                for message in rows.map(|i| &self.messages[i]) {
                    message.draw(ui);
                }

                if self.auto_scroll && received_message {
                    ui.scroll_to_cursor(Some(Align::BOTTOM));
                }
            });
        });
    }
}
