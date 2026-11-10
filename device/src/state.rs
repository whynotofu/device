use crate::{
    file_var::{FileVariable, Mode},
    stack_string::StackString,
    wrappers::FilePath,
};
use device_common::{BatteryProtection, KeyboardBacklight, Percentage, PlatformProfile};
use std::str::FromStr;

const STATE_FILE: &str = "/var/lib/device.state";

type StateString = StackString<256>;

pub struct StateManager {
    state_file: FileVariable<StateString>,
    state_on_file: State,
}

impl StateManager {
    pub fn init() -> StateManager {
        let state_file = FileVariable::<StateString>::new(&FilePath::from(STATE_FILE), Mode::Create);
        if !state_file.lock() {
            eprintln!("Another instance of this process is already running!");
            std::process::exit(1);
        }
        let state_on_file = State::from(&state_file.get());
        Self {
            state_file,
            state_on_file,
        }
    }

    pub fn get_state_on_file(&self) -> &State {
        &self.state_on_file
    }

    pub fn sync(&self, current_state: &State) {
        if self.state_on_file != *current_state {
            self.state_file.set(&StateString::from(current_state));
        }
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
pub struct State {
    pub battery_protection: Option<BatteryProtection>,
    pub drop_threshold: Option<bool>,
    pub display_brightness: Option<Percentage>,
    pub keyboard_backlight: Option<KeyboardBacklight>,
    pub platform_profile: Option<PlatformProfile>,
}

impl From<&StateString> for State {
    fn from(s: &StateString) -> Self {
        let mut state = State::default();
        for line in s.as_str().split('\n') {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "battery_protection" => {
                        state.battery_protection = BatteryProtection::from_str(value).ok();
                    }
                    "drop_threshold" => {
                        state.drop_threshold = match value {
                            "true" => Some(true),
                            "false" => Some(false),
                            _ => None,
                        };
                    }
                    "display_brightness" => {
                        state.display_brightness = Percentage::from_str(value).ok();
                    }
                    "keyboard_backlight" => {
                        state.keyboard_backlight = KeyboardBacklight::from_str(value).ok();
                    }
                    "platform_profile" => {
                        state.platform_profile = PlatformProfile::from_str(value).ok();
                    }
                    _ => {}
                }
            }
        }
        state
    }
}

impl From<&State> for StateString {
    fn from(state: &State) -> Self {
        let mut s = StateString::new();
        if let Some(battery_protection) = state.battery_protection {
            s.write("battery_protection = ");
            s.write(battery_protection.as_str());
            s.write("\n");
        };
        if let Some(drop_threshold) = state.drop_threshold {
            s.write("drop_threshold = ");
            s.write(if drop_threshold { "true" } else { "false" });
            s.write("\n");
        };
        if let Some(display_brightness) = state.display_brightness {
            s.write("display_brightness = ");
            s.write(&display_brightness.to_string().as_str());
            s.write("\n");
        };
        if let Some(keyboard_backlight) = state.keyboard_backlight {
            s.write("keyboard_backlight = ");
            s.write(keyboard_backlight.as_str());
            s.write("\n");
        };
        if let Some(platform_profile) = state.platform_profile {
            s.write("platform_profile = ");
            s.write(platform_profile.as_str());
            s.write("\n");
        };
        s
    }
}
