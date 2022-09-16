use enum_iterator::all;

use crate::{
    adapter::Port,
    calibration::{StickCalibration, TriggerCalibration, NOTCHES},
    config::{Config, Profile, DEFAULT_PROFILE_NAME},
    gui::{
        util::{enum_combo_ui, enum_option_combo_ui, no_close_popup_below_widget},
        ERROR_COLOR,
    },
};

const U8_TEXT_WIDTH: f32 = 30.0;

pub struct ConfigEditor<'a> {
    config: &'a mut Config,
    state: State,
}

impl<'a> ConfigEditor<'a> {
    pub fn new(config: &'a mut Config, state: Option<State>) -> Self {
        Self {
            config,
            state: state.map(|s| s.reset()).unwrap_or_default(),
        }
    }

    pub fn into_state(self) -> State {
        self.state
    }

    // TODO: Add config / profile editor dirtying. Add 'Save' and 'Reload' buttons to profile editor.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let State {
            name_and_profile,
            add_state,
            profile_action,
            reload,
            save,
            dirty,
        } = &mut self.state;

        if let Some(action) = profile_action.take() {
            match action {
                ProfileAction::Add(name, profile) => {
                    self.config.profile.list.insert(name.clone(), profile);
                    *name_and_profile = Some((name, profile));
                }
                ProfileAction::Remove(name) => {
                    self.config.profile.list.remove(&name);
                    *name_and_profile = None;
                }
            }
        }

        ui.horizontal(|ui| {
            if *dirty {
                ui.heading("Config*");
            } else {
                ui.heading("Config");
            }

            ui.separator();

            if ui.button("Save").clicked() {
                *save = true;
            }

            if ui.button("Reload").clicked() {
                // TODO: Ask for confirmation before reloading if profile is dirty.
                *reload = true;
            }
        });

        ui.separator();

        let scroll_contents = |ui: &mut egui::Ui| {
            ui.label("Active Profile");
            for p in all::<Port>() {
                egui::ComboBox::from_label(format!("Port {:?}", p))
                    .selected_text(&self.config.profile.selected[p.index()])
                    .show_ui(ui, |ui| {
                        for (name, _) in self.config.profile.list.iter() {
                            ui.selectable_value(
                                &mut self.config.profile.selected[p.index()],
                                name.clone(),
                                name.as_str(),
                            );
                        }
                    });
            }

            ui.separator();

            ui.label("Input Server");
            egui::Grid::new("input-server")
                .num_columns(4)
                .show(ui, |ui| {
                    for p in all::<Port>() {
                        let input_server = &mut self.config.input_server[p.index()];
                        ui.label(format!("Port {:?}", p));
                        ui.checkbox(&mut input_server.enabled, "Enabled");

                        let mut port_buf = format!("{}", input_server.port);
                        if ui.text_edit_singleline(&mut port_buf).changed() {
                            if port_buf.is_empty() {
                                input_server.port = 0;
                            } else if let Ok(p) = port_buf.parse::<u16>() {
                                input_server.port = p;
                            }
                        }

                        ui.label("UDP Port");
                        ui.end_row();
                    }
                });

            ui.separator();

            if name_and_profile.is_none() {
                *name_and_profile = self
                    .config
                    .profile
                    .list
                    .get_key_value(DEFAULT_PROFILE_NAME)
                    .map_or_else(|| self.config.profile.list.iter().next(), Option::Some)
                    .map(|(n, p)| (n.clone(), *p));
            }

            ui.group(|ui| {
                ui.label("Profile Editor");

                let profile_combo = {
                    let c = egui::ComboBox::from_label("Profile");
                    if let Some((name, _)) = name_and_profile {
                        c.selected_text(name.clone())
                    } else {
                        c
                    }
                };

                let selected = {
                    let mut selected_buf = name_and_profile
                        .as_ref()
                        .map(|t| t.0.clone())
                        .unwrap_or_default();

                    let response = profile_combo.show_ui(ui, |ui| {
                        for (name, _) in self.config.profile.list.iter() {
                            ui.selectable_value(&mut selected_buf, name.clone(), name);
                        }
                    });

                    if response.response.changed() {
                        Some(selected_buf)
                    } else {
                        None
                    }
                };

                ui.horizontal(|ui| {
                    let add_popup_id = ui.make_persistent_id("add");
                    let add_response = ui.button("Add");
                    if add_response.clicked() {
                        ui.memory().toggle_popup(add_popup_id);
                    }

                    no_close_popup_below_widget(ui, add_popup_id, &add_response, |ui| {
                        let mut add_popup = AddPopup::new(
                            add_state.take().unwrap_or_default(),
                            self.config,
                            name_and_profile,
                        );
                        *profile_action = add_popup.update(ui);
                        *add_state = Some(add_popup.into_state());
                    });

                    if !ui.memory().is_popup_open(add_popup_id) {
                        *add_state = None;
                    }

                    let remove_popup_id = ui.make_persistent_id("remove");
                    let remove_response = ui.button("Remove");
                    if remove_response.clicked() {
                        ui.memory().toggle_popup(remove_popup_id);
                    }

                    no_close_popup_below_widget(ui, remove_popup_id, &remove_response, |ui| {
                        if let Some((name, _)) = name_and_profile.as_ref() {
                            let mut remove_popup =
                                RemovePopup::new(name, self.config.profile.list.len() <= 1);
                            *profile_action = remove_popup.update(ui);
                        }
                    });
                });

                ui.separator();

                if let Some((name, profile)) = name_and_profile {
                    if let Some(selected) = selected {
                        *profile = self
                            .config
                            .profile
                            .list
                            .get(&selected)
                            .cloned()
                            .expect("selected profile should exist");
                        *name = selected;
                    }

                    enum_combo_ui(&mut profile.driver, "Driver", ui);
                    enum_combo_ui(&mut profile.rumble, "Rumble", ui);

                    ui.horizontal(|ui| {
                        ui.scope(|ui| {
                            ui.set_max_width(ui.available_width() * 0.33);

                            let mut buf = format!("{:.2}", profile.analog_scale);
                            if ui.text_edit_singleline(&mut buf).changed() {
                                if buf.is_empty() {
                                    profile.analog_scale = 0.0;
                                } else if let Ok(scale) = buf.parse::<f64>() {
                                    profile.analog_scale = scale;
                                }
                            }
                        });

                        ui.label("Analog Scale");
                    });

                    ui.group(|ui| {
                        ui.label("ViGEm");

                        enum_combo_ui(&mut profile.vigem_config.pad, "Pad", ui);
                        enum_combo_ui(&mut profile.vigem_config.trigger_mode, "Trigger Mode", ui);
                    });

                    ui.group(|ui| {
                        ui.label("Calibration");

                        ui.checkbox(&mut profile.calibration.enabled, "Enabled");

                        let stick_calibration_ui =
                            |c: &mut StickCalibration, id: usize, ui: &mut egui::Ui| {
                                const NOTCH_IDENT: [&str; NOTCHES] =
                                    ["T", "TR", "R", "BR", "B", "BL", "L", "TL"];

                                ui.push_id(id, |ui| {
                                    ui.label("Notch Points");
                                    egui::ScrollArea::horizontal().show(ui, |ui| {
                                        egui::Grid::new("notch-points").num_columns(NOTCHES).show(
                                            ui,
                                            |ui| {
                                                for row in 0..c.notch_points[0].len() + 1 {
                                                    for (col, ident) in
                                                        c.notch_points.iter_mut().zip(NOTCH_IDENT)
                                                    {
                                                        if row == 0 {
                                                            ui.label(ident);
                                                        } else {
                                                            ui.scope(|ui| {
                                                                ui.set_max_width(U8_TEXT_WIDTH);
                                                                let val = &mut col[row - 1];
                                                                let mut buf = format!("{}", *val);
                                                                if ui
                                                                    .text_edit_singleline(&mut buf)
                                                                    .clicked()
                                                                {
                                                                    if buf.is_empty() {
                                                                        *val = 0;
                                                                    } else if let Ok(n) =
                                                                        buf.parse::<u8>()
                                                                    {
                                                                        *val = n;
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    }

                                                    ui.end_row();
                                                }
                                            },
                                        );
                                    });

                                    ui.label("Center");
                                    for i in 0..c.center.len() {
                                        ui.scope(|ui| {
                                            ui.set_max_width(U8_TEXT_WIDTH);
                                            let val = &mut c.center[i];
                                            let mut buf = format!("{}", *val);
                                            if ui.text_edit_singleline(&mut buf).clicked() {
                                                if buf.is_empty() {
                                                    *val = 0;
                                                } else if let Ok(n) = buf.parse::<u8>() {
                                                    *val = n;
                                                }
                                            }
                                        });
                                    }
                                });
                            };

                        let trigger_calibration_ui =
                            |c: &mut TriggerCalibration, id: usize, ui: &mut egui::Ui| {
                                ui.push_id(id, |ui| {
                                    let mut first = true;

                                    loop {
                                        let (val, name) = if first {
                                            (&mut c.min, "Min")
                                        } else {
                                            (&mut c.max, "Max")
                                        };

                                        ui.horizontal(|ui| {
                                            ui.scope(|ui| {
                                                ui.set_max_width(U8_TEXT_WIDTH);
                                                let mut buf = format!("{}", *val);
                                                if ui.text_edit_singleline(&mut buf).changed() {
                                                    if buf.is_empty() {
                                                        *val = 0;
                                                    } else if let Ok(n) = buf.parse::<u8>() {
                                                        *val = n;
                                                    }
                                                }
                                            });

                                            ui.label(name);
                                        });

                                        if first {
                                            first = false;
                                        } else {
                                            break;
                                        }
                                    }
                                });
                            };

                        if let Some(c) = profile.calibration.stick_data.as_mut() {
                            ui.label("Main Stick");
                            stick_calibration_ui(&mut c.main_stick, 0, ui);

                            ui.label("C-Stick");
                            stick_calibration_ui(&mut c.c_stick, 1, ui);
                        } else {
                            ui.horizontal(|ui| {
                                ui.label("Missing stick calibration data");
                                if ui.button("Use default").clicked() {
                                    profile.calibration.stick_data = Some(Default::default());
                                }
                            });
                        }

                        if let Some(c) = profile.calibration.trigger_data.as_mut() {
                            ui.label("Left Trigger");
                            trigger_calibration_ui(&mut c.left_trigger, 0, ui);

                            ui.label("Right Trigger");
                            trigger_calibration_ui(&mut c.right_trigger, 1, ui);
                        } else {
                            ui.horizontal(|ui| {
                                ui.label("Missing trigger calibration data");
                                if ui.button("Use default").clicked() {
                                    profile.calibration.trigger_data = Some(Default::default());
                                }
                            });
                        }
                    });

                    enum_option_combo_ui(&mut profile.ess.inversion_mapping, "Ess Inversion", ui);
                } else {
                    ui.label("No profile");
                }
            });
        };

        egui::ScrollArea::vertical().show(ui, scroll_contents);
    }
}

#[derive(Default)]
pub struct State {
    name_and_profile: Option<(String, Profile)>,
    add_state: Option<AddState>,
    profile_action: Option<ProfileAction>,
    reload: bool,
    save: bool,
    dirty: bool,
}

impl State {
    pub fn profile_mut(&mut self) -> Option<&mut Profile> {
        self.name_and_profile.as_mut().map(|t| &mut t.1)
    }

    pub fn reload(&self) -> bool {
        self.reload
    }

    pub fn save(&self) -> bool {
        self.save
    }

    fn reset(mut self) -> Self {
        self.reload = false;
        self.save = false;
        self
    }
}

pub enum ProfileAction {
    Add(String, Profile),
    Remove(String),
}

struct AddPopup<'a> {
    state: AddState,
    profile_action: Option<ProfileAction>,
    config: &'a Config,
    name_and_profile: &'a Option<(String, Profile)>,
}

impl<'a> AddPopup<'a> {
    pub fn new(
        state: AddState,
        config: &'a Config,
        name_and_profile: &'a Option<(String, Profile)>,
    ) -> Self {
        Self {
            state,
            profile_action: None,
            config,
            name_and_profile,
        }
    }

    pub fn into_state(self) -> AddState {
        self.state
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let AddPopup {
            state: AddState { name, action },
            profile_action,
            config,
            name_and_profile,
        } = self;

        let mut next_action = None;

        let set_profile_action = |profile_action: &mut Option<ProfileAction>, name: &str| {
            *profile_action = Some(ProfileAction::Add(
                name.to_owned(),
                name_and_profile.as_ref().map(|t| t.1).unwrap_or_default(),
            ));
        };

        ui.set_min_width(200.0);

        match action {
            AddAction::Input { err } => {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(name);
                    ui.label("Name");
                });

                if let Some(err) = err.as_ref() {
                    ui.colored_label(ERROR_COLOR, err.as_str());
                }

                ui.horizontal(|ui| {
                    let add = ui.button("Add");
                    let cancel = ui.button("Cancel");

                    if add.clicked() {
                        if name.is_empty() {
                            *err = Some("Profile name must not be empty".to_owned());
                        } else if config.profile.list.contains_key(name) {
                            next_action = Some(AddAction::AlreadyExists);
                        } else {
                            set_profile_action(profile_action, name);
                        }
                    }

                    if (profile_action.is_some() && add.clicked()) || cancel.clicked() {
                        ui.memory().close_popup();
                    }
                });
            }
            AddAction::AlreadyExists => {
                ui.label(format!("Profile '{}' already exists, replace it?", name));
                ui.colored_label(ERROR_COLOR, "The original profile will be lost.");

                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        set_profile_action(profile_action, name);
                        ui.memory().close_popup();
                    }

                    if ui.button("No").clicked() {
                        ui.memory().close_popup();
                    }
                });
            }
        }

        if let Some(next) = next_action {
            *action = next;
        }
    }

    #[must_use]
    pub fn update(&mut self, ui: &mut egui::Ui) -> Option<ProfileAction> {
        self.ui(ui);
        self.profile_action.take()
    }
}

#[derive(Default)]
struct AddState {
    name: String,
    action: AddAction,
}

enum AddAction {
    Input { err: Option<String> },
    AlreadyExists,
}

impl Default for AddAction {
    fn default() -> Self {
        Self::Input { err: None }
    }
}

struct RemovePopup<'a> {
    name: &'a str,
    is_last_profile: bool,
    profile_action: Option<ProfileAction>,
}

impl<'a> RemovePopup<'a> {
    pub fn new(name: &'a str, is_last_profile: bool) -> Self {
        Self {
            name,
            is_last_profile,
            profile_action: None,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);

        if self.is_last_profile {
            ui.label("Cannot remove all profiles.");

            ui.horizontal(|ui| {
                if ui.button("Ok").clicked() {
                    ui.memory().close_popup();
                }
            });
        } else {
            ui.label(format!("Remove profile '{}'?", self.name));
            ui.colored_label(ERROR_COLOR, "The profile will be lost.");

            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    self.profile_action = Some(ProfileAction::Remove(self.name.to_owned()));
                    ui.memory().close_popup();
                }

                if ui.button("No").clicked() {
                    ui.memory().close_popup();
                }
            });
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> Option<ProfileAction> {
        self.ui(ui);
        self.profile_action.take()
    }
}
