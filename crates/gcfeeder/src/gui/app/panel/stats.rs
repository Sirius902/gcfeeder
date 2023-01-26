use enum_iterator::all;
use gcfeeder_core::{
    adapter::{source::InputSource, Port},
    feeder::Feeder,
};

pub struct StatsPanel<'a, S: InputSource + 'static> {
    input_source: &'a mut S,
    feeders: &'a mut [Feeder<S::Listener>; Port::COUNT],
}

impl<'a, S: InputSource> StatsPanel<'a, S> {
    pub fn new(
        input_source: &'a mut S,
        feeders: &'a mut [Feeder<S::Listener>; Port::COUNT],
    ) -> Self {
        Self {
            input_source,
            feeders,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);

        let poll_avg = self
            .input_source
            .average_poll_time()
            .filter(|_| self.input_source.connected())
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
