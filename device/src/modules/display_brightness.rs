use crate::{
    file_var::{FileVariable, Mode},
    listeners::file::{FileListener, Tag},
    wrappers::{File, FilePath},
};
use device_common::Percentage;

const BRIGHTNESS_DEVS: [&str; 4] = ["intel_backlight", "dp_aux_backlight", "amdgpu_bl0", "nv_backlight"];

pub struct DisplayBrightnessModule {
    brightness_file: FileVariable<u32>,
    step: u32,
}

impl DisplayBrightnessModule {
    pub const DEFAULT_DISPLAY_BRIGHTNESS: u8 = 50;

    pub fn new(brightness: Percentage, file_listener: &mut FileListener) -> Option<DisplayBrightnessModule> {
        Self::find().map(|name| Self::init(name, brightness, file_listener))
    }

    fn init(name: &str, brightness: Percentage, file_listener: &mut FileListener) -> DisplayBrightnessModule {
        let mut display_brightness = DisplayBrightnessModule {
            brightness_file: FileVariable::<u32>::new(&Self::brightness_file(name), Mode::ReadWrite),
            step: FileVariable::<u32>::static_get(&Self::max_brightness_file(name)) / 100,
        };

        display_brightness.set_brightness(brightness);
        file_listener.add_file(&Self::brightness_file(name), Tag::DisplayBrightness);
        println!("Display Brightness: {}", name);
        display_brightness
    }

    pub fn get_brightness(&self) -> Percentage {
        Percentage::from((self.brightness_file.get() / self.step) as u8)
    }

    pub fn set_brightness(&mut self, brightness: Percentage) {
        self.brightness_file.set(&((brightness.value() as u32) * self.step));
    }

    fn is_valid(name: &str) -> bool {
        File::exist(&Self::brightness_file(name))
            && File::exist(&Self::max_brightness_file(name))
            && Self::is_max_brightness_valid(FileVariable::<u32>::static_get(&Self::max_brightness_file(name)))
    }

    fn is_max_brightness_valid(n: u32) -> bool {
        n.is_multiple_of(100)
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
