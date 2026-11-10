use crate::{
    config::Config,
    device::Listeners,
    file_var::{FileVariable, Mode},
    listeners::{event::Event, file::Tag},
    server::Server,
    state::State,
    timer::Timer,
    wrappers::{File, FilePath},
};
use device_common::{Item, KeyboardBacklight, UNum};
use std::{fs, path::Path, time::Duration};

pub struct KeyboardBacklightModule {
    brightness_file: FileVariable<u32>,
    step: u32,
    brightness: KeyboardBacklight,
    poller: Timer,
}

impl KeyboardBacklightModule {
    pub fn new(config: &Config, state: &State, listeners: &mut Listeners) -> Option<KeyboardBacklightModule> {
        if let Some(name) = &config.keyboard_backlight {
            let name = name.as_str();
            if Self::is_valid(name) {
                Some(Self::init(name, state, listeners))
            } else {
                println!("Warning: Unable to use keyboard backlight device specified in config");
                None
            }
        } else {
            Self::find().map(|name| Self::init(name.as_str(), state, listeners))
        }
    }

    fn init(name: &str, state: &State, listeners: &mut Listeners) -> KeyboardBacklightModule {
        let brightness_file = FileVariable::<u32>::new(&Self::brightness_file(name), Mode::ReadWrite);
        let step = (FileVariable::<u32>::static_get(&Self::max_brightness_file(name)) + 1) / 4;
        let brightness = KeyboardBacklight::from((brightness_file.get() / step) as UNum);
        let poller = Timer::new();
        let mut keyboard_backlight = KeyboardBacklightModule {
            brightness_file,
            step,
            brightness,
            poller,
        };
        if let Some(brightness) = state.keyboard_backlight {
            keyboard_backlight.set_brightness(brightness);
        }
        keyboard_backlight.poller.set(Duration::from_millis(50), Some(10));
        listeners.file.add(&Self::brightness_file(name), Tag::KeyboardBacklight);
        if File::exist(&Self::brightness_hw_changed_file(name)) {
            listeners.file.add(&Self::brightness_hw_changed_file(name), Tag::KeyboardBacklight);
        }
        listeners.event.add(keyboard_backlight.poller.get_fd(), Event::KeyboardBacklightPoll);
        println!("Keyboard Backlight: {}", name);
        keyboard_backlight
    }

    pub fn get_brightness(&self) -> KeyboardBacklight {
        self.brightness
    }

    pub fn set_brightness(&mut self, brightness: KeyboardBacklight) {
        self.brightness_file.set(&((brightness.value() as u32) * self.step));
    }

    pub fn start_polling(&mut self) {
        self.poller.start();
    }

    pub fn poll(&mut self, server: &Server) {
        self.poller.update();
        let brightness = KeyboardBacklight::from((self.brightness_file.get() / self.step) as UNum);
        if self.brightness != brightness {
            self.poller.stop();
            self.brightness = brightness;
            server.broadcast(Item::KeyboardBacklight(brightness));
        }
    }

    fn is_valid(name: &str) -> bool {
        File::exist(&Self::brightness_file(name))
            && File::exist(&Self::max_brightness_file(name))
            && Self::is_max_brightness_valid(FileVariable::<u32>::static_get(&Self::max_brightness_file(name)))
    }

    fn is_max_brightness_valid(n: u32) -> bool {
        (2..10).any(|e| (n + 1) == (1 << e))
    }

    fn find() -> Option<String> {
        let dir = Path::new("/sys/class/leds/");
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.ends_with("::kbd_backlight")
                    && let Some(name) = name.strip_suffix("::kbd_backlight")
                    && Self::is_valid(name)
                {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    fn brightness_file(name: &str) -> FilePath {
        Self::make_keyboard_backlight_path(name, "brightness")
    }

    fn brightness_hw_changed_file(name: &str) -> FilePath {
        Self::make_keyboard_backlight_path(name, "brightness_hw_changed")
    }

    fn max_brightness_file(name: &str) -> FilePath {
        Self::make_keyboard_backlight_path(name, "max_brightness")
    }

    fn make_keyboard_backlight_path(name: &str, file: &str) -> FilePath {
        FilePath::from("/sys/class/leds/").add(name).add("::kbd_backlight/").add(file)
    }
}
