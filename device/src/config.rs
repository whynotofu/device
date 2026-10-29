use serde::{Deserialize, Deserializer};
use std::{fs, path::Path};

const CONFIG_FILE: &str = "/etc/device.toml";

#[derive(Deserialize)]
pub struct StaticConfig {
    pub intel_turbo: Option<bool>,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    keyboard_backlight: Option<String>,
    #[serde(deserialize_with = "validate_battery_polling_interval")]
    battery_polling_interval: u8,
    static_config: Option<StaticConfig>,
}

fn validate_battery_polling_interval<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let value = u8::deserialize(deserializer)?;
    let options = [5, 10, 15, 20, 30, 60];

    if options.contains(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "Valid battery_polling_interval values are: {:?}; using default",
            options
        )))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keyboard_backlight: None,
            battery_polling_interval: 10,
            static_config: None,
        }
    }
}

impl Config {
    pub fn init() -> Config {
        let config_file = Path::new(CONFIG_FILE);

        if config_file.exists() {
            match fs::read_to_string(config_file) {
                Ok(content) => match toml::from_str::<Config>(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("Failed to deserialize config: {}", e)
                    }
                },
                Err(e) => eprintln!("Failed to load config: {}", e),
            };
        }

        Self::default()
    }

    pub fn get_keyboard_backlight(&self) -> Option<&String> {
        self.keyboard_backlight.as_ref()
    }

    pub fn get_battery_polling_interval(&self) -> u8 {
        self.battery_polling_interval
    }

    pub fn get_static_config(&self) -> Option<&StaticConfig> {
        self.static_config.as_ref()
    }
}
