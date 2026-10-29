use crate::{
    config::StaticConfig,
    file_var::{FileVariable, Mode},
    modules::{battery::BatteryModule, display_brightness::DisplayBrightnessModule},
    stack_string::StackString,
    wrappers::FilePath,
};
use device_common::{BatteryProtection, KeyboardBacklight, Percentage, PlatformProfile};

const STATE_FILE: &str = "/var/lib/device.state";

type StateString = StackString<10>;

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

    pub fn apply_static_config(static_config: &StaticConfig) {
        if let Some(intel_turbo) = static_config.intel_turbo {
            FileVariable::<u8>::static_set(
                &FilePath::from("/sys/devices/system/cpu/intel_pstate/no_turbo"),
                &(!intel_turbo as u8),
            );
        }
    }

    pub fn get_state_on_file(&self) -> State {
        self.state_on_file
    }

    pub fn sync(&self, current_state: &State) {
        if self.state_on_file != *current_state {
            self.state_file.set(&StateString::from(current_state));
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct State {
    pub battery_protection: BatteryProtection,
    pub drop_threshold: bool,
    pub display_brightness: Percentage,
    pub keyboard_backlight: KeyboardBacklight,
    pub platform_profile: PlatformProfile,
}

impl From<&StateString> for State {
    fn from(s: &StateString) -> Self {
        match u64::from_str_radix(s.as_str(), 16) {
            Ok(n) => {
                let bytes = n.to_le_bytes();
                Self {
                    battery_protection: BatteryProtection::try_from(bytes[4]).unwrap_or_default(),
                    drop_threshold: bytes[3] > 0,
                    display_brightness: Percentage::from(bytes[2]),
                    keyboard_backlight: KeyboardBacklight::from(bytes[1]),
                    platform_profile: PlatformProfile::try_from(bytes[0]).unwrap_or_default(),
                }
            }
            Err(_e) => Self {
                battery_protection: BatteryProtection::default(),
                drop_threshold: BatteryModule::DEFAULT_DROP_THRESHOLD,
                display_brightness: Percentage::from(DisplayBrightnessModule::DEFAULT_DISPLAY_BRIGHTNESS),
                keyboard_backlight: KeyboardBacklight::default(),
                platform_profile: PlatformProfile::default(),
            },
        }
    }
}

impl From<&State> for StateString {
    fn from(state: &State) -> Self {
        let mut bytes = [0u8; 5];
        bytes[0] = state.battery_protection.code();
        bytes[1] = state.drop_threshold as u8;
        bytes[2] = state.display_brightness.value();
        bytes[3] = state.keyboard_backlight.value();
        bytes[4] = state.platform_profile.code();
        StateString::from(bytes_as_hex_bytes(&bytes))
    }
}

fn bytes_as_hex_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    let mut hex_bytes = [0u8; N];
    for i in 0..N {
        let nibble = (bytes[i / 2] >> ((i + 1) % 2 * 4)) & 0x0F;
        hex_bytes[i] = HEX_CHARS[nibble as usize];
    }
    hex_bytes
}
