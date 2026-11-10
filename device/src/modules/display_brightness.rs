use crate::{
    device::Listeners,
    file_var::{FileVariable, Mode},
    listeners::file::Tag,
    state::State,
    wrappers::{File, FilePath},
};
use device_common::Percentage;

const BRIGHTNESS_DEVS: [&str; 4] = ["intel_backlight", "dp_aux_backlight", "amdgpu_bl0", "nv_backlight"];

pub struct DisplayBrightnessModule {
    brightness_file: FileVariable<u32>,
    max_brightness: u32,
}

impl DisplayBrightnessModule {
    pub fn new(state: &State, listeners: &mut Listeners) -> Option<DisplayBrightnessModule> {
        Self::find().map(|name| Self::init(name, state, listeners))
    }

    fn init(name: &str, state: &State, listeners: &mut Listeners) -> DisplayBrightnessModule {
        let mut display_brightness = DisplayBrightnessModule {
            brightness_file: FileVariable::<u32>::new(&Self::brightness_file(name), Mode::ReadWrite),
            max_brightness: FileVariable::<u32>::static_get(&Self::max_brightness_file(name)),
        };
        if let Some(brightness) = state.display_brightness {
            display_brightness.set_brightness(brightness);
        }
        listeners.file.add(&Self::brightness_file(name), Tag::DisplayBrightness);
        println!("Display Brightness: {}", name);
        display_brightness
    }

    pub fn get_brightness(&self) -> Percentage {
        Percentage::from(((self.brightness_file.get() as f32) / (self.max_brightness as f32)) * 100.0)
    }

    pub fn set_brightness(&mut self, brightness: Percentage) {
        self.brightness_file.set(&(((self.max_brightness as f32) * brightness.normalized()) as u32));
    }

    fn is_valid(name: &str) -> bool {
        File::exist(&Self::brightness_file(name)) && File::exist(&Self::max_brightness_file(name))
    }

    fn find() -> Option<&'static str> {
        BRIGHTNESS_DEVS.into_iter().find(|&name| Self::is_valid(name))
    }

    fn brightness_file(name: &str) -> FilePath {
        Self::make_display_brightness_path(name, "brightness")
    }

    fn max_brightness_file(name: &str) -> FilePath {
        Self::make_display_brightness_path(name, "max_brightness")
    }

    fn make_display_brightness_path(name: &str, file: &str) -> FilePath {
        FilePath::from("/sys/class/backlight/").add(name).add("/").add(file)
    }
}
