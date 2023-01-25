use crate::gui::app::{Source, Usb};
use enum_iterator::all;
use gcfeeder_core::{
    adapter::{poller::Poller, Port},
    feeder::Feeder,
};

pub struct StatsPanel<'a> {
    poller: &'a mut Poller<Usb>,
    feeders: &'a mut [Feeder<Source>; Port::COUNT],
}

impl<'a> StatsPanel<'a> {
    pub fn new(
        poller: &'a mut Poller<Usb>,
        feeders: &'a mut [Feeder<Source>; Port::COUNT],
    ) -> Self {
        Self { poller, feeders }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);

        let poll_avg = self
            .poller
            .average_poll_time()
            .filter(|_| self.poller.connected())
            .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "-".to_owned());

        ui.heading("Average poll time");
        ui.label(format!("{}ms", poll_avg));

        ui.add_space(5.0);

        ui.heading("Average feed time");

        for port in all::<Port>() {
            let feeder = &self.feeders[port.index()];

            let feed_avg = feeder
                .average_feed_time()
                .filter(|_| feeder.connected())
                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                .unwrap_or_else(|| "-".to_owned());

            ui.label(format!("Port {:?}: {}ms", port, feed_avg));
        }
    }
}
