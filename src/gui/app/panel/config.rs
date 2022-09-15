use enum_iterator::all;

use crate::{
    adapter::Port,
    config::{Config, Profile, DEFAULT_PROFILE_NAME},
    gui::util::{enum_combo_ui, enum_option_combo_ui, no_close_popup_below_widget},
};

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
                    self.config
                        .profile
                        .list
                        .insert(name.clone(), profile.clone());
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

                let mut selected = None;
                profile_combo.show_ui(ui, |ui| {
                    for (name, _) in self.config.profile.list.iter() {
                        ui.selectable_value(&mut selected, Some(name), name);
                    }
                });

                ui.horizontal(|ui| {
                    let add_popup_id = ui.make_persistent_id("add");
                    let add_response = ui.button("Add");
                    if add_response.clicked() {
                        ui.memory().toggle_popup(add_popup_id);
                    }

                    no_close_popup_below_widget(ui, add_popup_id, &add_response, |ui| {
                        let mut add_popup =
                            AddPopup::new(add_state.take().unwrap_or_default(), &name_and_profile);
                        *profile_action = add_popup.update(ui);
                        *add_state = Some(add_popup.into_state());
                    });

                    let remove_popup_id = ui.make_persistent_id("remove");
                    let remove_response = ui.button("Remove");
                    if remove_response.clicked() {
                        ui.memory().toggle_popup(remove_popup_id);
                    }

                    no_close_popup_below_widget(ui, remove_popup_id, &remove_response, |ui| {
                        let mut remove_popup = RemovePopup::new();
                        *profile_action = remove_popup.update(ui);
                    });
                });

                ui.separator();

                if let Some(selected) = selected {
                    *name_and_profile = self
                        .config
                        .profile
                        .list
                        .get_key_value(selected)
                        .map(|(n, p)| (n.clone(), p.clone()));

                    assert!(name_and_profile.is_some());
                } else if name_and_profile.is_none() {
                    *name_and_profile = self
                        .config
                        .profile
                        .list
                        .get_key_value(DEFAULT_PROFILE_NAME)
                        .map_or_else(|| self.config.profile.list.iter().next(), Option::Some)
                        .map(|(n, p)| (n.clone(), p.clone()));
                }

                if let Some((_, profile)) = name_and_profile {
                    enum_combo_ui(&mut profile.driver, "Driver", ui);
                    enum_combo_ui(&mut profile.rumble, "Rumble", ui);
                    enum_option_combo_ui(&mut profile.ess.inversion_mapping, "Ess Inversion", ui);
                } else {
                    ui.label("No profiles");
                }
            });
        };

        egui::ScrollArea::both().show(ui, scroll_contents);
    }
}

pub struct State {
    name_and_profile: Option<(String, Profile)>,
    add_state: Option<AddState>,
    profile_action: Option<ProfileAction>,
    reload: bool,
    save: bool,
    dirty: bool,
}

impl State {
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

impl Default for State {
    fn default() -> Self {
        Self {
            name_and_profile: None,
            add_state: None,
            profile_action: None,
            reload: false,
            save: false,
            dirty: false,
        }
    }
}

pub enum ProfileAction {
    Add(String, Profile),
    Remove(String),
}

struct AddPopup<'a> {
    state: AddState,
    profile_action: Option<ProfileAction>,
    name_and_profile: &'a Option<(String, Profile)>,
}

impl<'a> AddPopup<'a> {
    pub fn new(state: AddState, name_and_profile: &'a Option<(String, Profile)>) -> Self {
        Self {
            state,
            profile_action: None,
            name_and_profile,
        }
    }

    pub fn into_state(self) -> AddState {
        self.state
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let AddPopup {
            state: AddState {
                name,
                submitted_bad,
            },
            profile_action,
            name_and_profile,
        } = self;

        ui.set_min_width(200.0);

        ui.horizontal(|ui| {
            ui.text_edit_singleline(name);
            ui.label("Name");
        });

        ui.horizontal(|ui| {
            let is_bad_name = name.is_empty();
            let add = ui.button("Add");
            let cancel = ui.button("Cancel");

            if add.clicked() {
                *submitted_bad = is_bad_name;
                if !is_bad_name {
                    *profile_action = Some(ProfileAction::Add(
                        (*name).clone(),
                        name_and_profile
                            .as_ref()
                            .map(|t| t.1.clone())
                            .unwrap_or_default(),
                    ));
                }
            }

            if (!is_bad_name && add.clicked()) || cancel.clicked() {
                ui.memory().close_popup();
            }
        });
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
    submitted_bad: bool,
}

struct RemovePopup {
    profile_action: Option<ProfileAction>,
}

impl RemovePopup {
    pub fn new() -> Self {
        Self {
            profile_action: None,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(200.0);
        ui.label("remove wow");
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> Option<ProfileAction> {
        self.ui(ui);
        self.profile_action.take()
    }
}
