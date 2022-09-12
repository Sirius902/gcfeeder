use enum_iterator::all;

use crate::{
    adapter::{poller::Poller, Port},
    feeder::Feeder,
    gui::app::Usb,
};

pub struct StatsPanel<'a> {
    poller: &'a mut Poller<Usb>,
    feeders: &'a mut [Feeder<Usb>; Port::COUNT],
}

impl<'a> StatsPanel<'a> {
    pub fn new(poller: &'a mut Poller<Usb>, feeders: &'a mut [Feeder<Usb>; Port::COUNT]) -> Self {
        Self { poller, feeders }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);
        ui.heading("Stats");
        ui.add_space(5.0);

        let poll_avg = self
            .poller
            .average_poll_time()
            .filter(|_| self.poller.connected())
            .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "-".to_owned());

        ui.label(format!("Average poll time: {}ms", poll_avg));

        ui.spacing();

        for port in all::<Port>() {
            let feeder = &self.feeders[port.index()];

            let feed_avg = feeder
                .average_feed_time()
                .filter(|_| feeder.connected())
                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                .unwrap_or_else(|| "-".to_owned());

            ui.label(format!("Port {:?}", port));
            ui.label(format!("Average feed time: {}ms", feed_avg));
        }
    }
}
