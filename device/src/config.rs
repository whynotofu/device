use crate::{
    file_var::FileVariable,
    stack_string::StackString,
    wrappers::{File, FilePath},
};

const CONFIG_FILE: &str = "/etc/device.toml";

pub struct Config {
    pub keyboard_backlight: Option<String>,
    pub battery_polling_interval: u8,
    pub intel_turbo: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keyboard_backlight: None,
            battery_polling_interval: 10,
            intel_turbo: None,
        }
    }
}

impl Config {
    pub fn init() -> Config {
        let config = Self::load_config();

        let intel_turbo_file = FilePath::from("/sys/devices/system/cpu/intel_pstate/no_turbo");

        if let Some(intel_turbo) = config.intel_turbo
            && File::exist(&intel_turbo_file)
        {
            FileVariable::<u8>::static_set(&intel_turbo_file, &(!intel_turbo as u8));
        }

        config
    }

    fn load_config() -> Config {
        let config_file = FilePath::from(CONFIG_FILE);

        if File::exist(&config_file) {
            if File::size(&config_file) < 2048 {
                let content = FileVariable::<StackString<2048>>::static_get(&config_file);

                let mut config = Config::default();
                for line in content.as_str().split('\n') {
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        match key {
                            "battery_polling_interval" => {
                                let value = u8::from_str_radix(value, 10).unwrap_or_default();
                                let options = [5, 10, 15, 20, 30, 60];

                                if options.contains(&value) {
                                    config.battery_polling_interval = value;
                                } else {
                                    eprintln!(
                                        "WARN: Config: Valid battery_polling_interval values are: {:?}; using default",
                                        options
                                    );
                                }
                            }
                            "keyboard_backlight" => {
                                config.keyboard_backlight = Some(value.trim_matches('"').to_string());
                            }
                            "intel_turbo" => {
                                config.intel_turbo = match value {
                                    "true" => Some(true),
                                    "false" => Some(false),
                                    _ => None,
                                };
                            }
                            _ => {}
                        }
                    }
                }
                return config;
            } else {
                eprintln!("ERROR: Config file too large");
            }
        }

        Self::default()
    }
}
