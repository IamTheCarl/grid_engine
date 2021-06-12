// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of user inputs such as keyboard, mouse, controllers, etc.

use serde::{Deserialize, Serialize};
use std::{cmp::Eq, collections::HashMap, hash::Hash};

/// An ID used to quickly reference a control once it is past the physical representation.
pub type ControlID = std::num::NonZeroUsize;

/// An identifier for a physical control, such as a button on a controller or a joystick.
pub trait InputKey: std::hash::Hash + std::cmp::Eq {}

/// Structure for managing and mapping input configuration.
#[derive(Serialize, Deserialize)]
pub struct ControlManager<InputKey: Hash + Eq> {
    names_to_ids: HashMap<String, ControlID>,
    input_map: HashMap<InputKey, ControlID>,

    #[serde(skip)]
    input_state: Vec<f32>,
}

impl<InputKeyT: InputKey> ControlManager<InputKeyT> {
    pub fn build_control_manager(controls: &[&str]) -> ControlManager<InputKeyT> {
        let mut names_to_ids = HashMap::new();
        let input_map = HashMap::new();
        let input_state = vec![0f32; controls.len()];

        for name in controls {
            let id = names_to_ids.len() + 1;
            names_to_ids.insert(String::from(*name), ControlID::new(id).expect("Error generating control ID"));
        }

        ControlManager { names_to_ids, input_map, input_state }
    }

    pub fn get_control_id(&self, control_name: &str) -> Option<ControlID> {
        self.names_to_ids.get(control_name).cloned()
    }

    pub fn set_key_binding(&mut self, input_key: InputKeyT, control_id: ControlID) {
        self.input_map.insert(input_key, control_id);
    }

    pub fn update_input(&mut self, input_key: &InputKeyT, delta: f32) {
        if let Some(control_id) = self.input_map.get(input_key) {
            // This input actually has a control binding.
            self.input_state[control_id.get() - 1] = delta;
        }
    }

    pub fn get_boolean_control(&self, control_id: ControlID) -> bool {
        if let Some(state) = self.input_state.get(control_id.get() - 1) {
            return *state >= 1.0f32;
        } else {
            return false;
        }
    }

    pub fn get_analog_control(&self, control_id: ControlID) -> f32 {
        if let Some(state) = self.input_state.get(control_id.get() - 1) {
            return *state;
        } else {
            return 0.0f32;
        }
    }
}
