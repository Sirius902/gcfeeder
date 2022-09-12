use enum_iterator::all;

use crate::{adapter::Port, config::Config};

pub struct ConfigEditor<'a> {
    config: &'a mut Config,
    state: State,
}

impl<'a> ConfigEditor<'a> {
    pub fn new(config: &'a mut Config) -> Self {
        Self {
            config,
            state: State::default(),
        }
    }

    pub fn into_state(self) -> State {
        self.state
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Config");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.button("Reload").clicked() {
                self.state.reload = true;
            }

            if ui.button("Save").clicked() {
                self.state.save = true;
            }
        });

        ui.separator();

        for port in all::<Port>() {
            ui.label(format!(
                "Profile for Port {:?}: {}",
                port,
                self.config.profile.selected[port.index()]
            ));
        }
    }
}

#[derive(Default)]
pub struct State {
    reload: bool,
    save: bool,
}

impl State {
    pub fn reload(&self) -> bool {
        self.reload
    }

    pub fn save(&self) -> bool {
        self.save
    }
}
