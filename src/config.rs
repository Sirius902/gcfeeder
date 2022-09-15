use std::{array, collections::HashMap};

use serde::{Deserialize, Serialize};

use crate::{adapter::Port, feeder};

pub type Profile = feeder::Config;

pub const DEFAULT_PROFILE_NAME: &str = "default";

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub profile: ProfileConfig,
    pub input_server: [InputServerConfig; Port::COUNT],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: Default::default(),
            input_server: array::from_fn(|i| {
                InputServerConfig::new_disabled(4096 + u16::try_from(i).unwrap())
            }),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub selected: [String; Port::COUNT],
    pub list: HashMap<String, Profile>,
}

impl ProfileConfig {
    pub fn selected(&self, port: Port) -> Option<&Profile> {
        let name = &self.selected[port.index()];
        self.list.get(name)
    }

    pub fn selected_mut(&mut self, port: Port) -> Option<&mut Profile> {
        let name = &self.selected[port.index()];
        self.list.get_mut(name)
    }
}

impl Default for ProfileConfig {
    fn default() -> Self {
        let default = DEFAULT_PROFILE_NAME.to_owned();
        Self {
            selected: array::from_fn(|_| default.clone()),
            list: {
                let mut p = HashMap::new();
                p.insert(default, Profile::default());
                p
            },
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct InputServerConfig {
    pub enabled: bool,
    pub port: u16,
}

impl InputServerConfig {
    pub const fn new_disabled(port: u16) -> Self {
        Self {
            enabled: false,
            port,
        }
    }
}
