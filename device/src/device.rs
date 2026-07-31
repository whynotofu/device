use crate::{
    config::Config,
    listeners::{
        event::{Event, EventListener},
        file::{FileListener, Tag},
    },
    modules::{
        battery::BatteryModule, display_brightness::DisplayBrightnessModule, keyboard_backlight::KeyboardBacklightModule,
        platform_profile::PlatformProfileModule,
    },
    server::Server,
    state::State,
};
use device_common::{
    BatteryProtection, BatteryStatus, KeyboardBacklight, Message, Percentage, PlatformProfile, PlatformProfileChoices,
};

pub struct Device {
    battery: Option<BatteryModule>,
    display_brightness: Option<DisplayBrightnessModule>,
    keyboard_backlight: Option<KeyboardBacklightModule>,
    platform_profile: Option<PlatformProfileModule>,
    file_listener: FileListener,
}

impl Device {
    pub fn init(config: &Config, event_listener: &mut EventListener, state: &State) -> Self {
        let mut file_listener = FileListener::init(4);

        let battery = BatteryModule::new(
            config.get_battery_polling_interval(),
            state.battery_protection,
            state.drop_threshold,
        );
        let display_brightness = DisplayBrightnessModule::new(state.display_brightness, &mut file_listener);
        let keyboard_backlight =
            KeyboardBacklightModule::new(config.get_keyboard_backlight(), state.keyboard_backlight, &mut file_listener);
        let platform_profile = PlatformProfileModule::new(state.platform_profile, &mut file_listener);

        if battery.is_none() && display_brightness.is_none() && keyboard_backlight.is_none() && platform_profile.is_none() {
            eprintln!("No modules initialized!");
            std::process::exit(0);
        }

        if let Some(battery) = &battery {
            event_listener.add_event_source(battery.get_netlink_fd(), Event::PowerSupply);
            event_listener.add_event_source(battery.get_timer_fd(), Event::BatteryPoll);
        }

        if let Some(keyboard_backlight) = &keyboard_backlight {
            event_listener.add_event_source(keyboard_backlight.get_timer_fd(), Event::KeyboardBacklightPoll);
        }

        event_listener.add_event_source(file_listener.get_fd(), Event::FileChange);

        Self {
            battery,
            display_brightness,
            keyboard_backlight,
            platform_profile,
            file_listener,
        }
    }

    pub fn has_battery(&self) -> bool {
        self.battery.is_some()
    }

    pub fn get_battery_charge(&self) -> Option<Percentage> {
        self.battery.as_ref().map(|battery| battery.get_charge())
    }

    pub fn get_battery_status(&self) -> Option<BatteryStatus> {
        self.battery.as_ref().map(|battery| battery.get_status())
    }

    pub fn has_battery_protection(&self) -> bool {
        if let Some(battery) = &self.battery {
            battery.has_protection()
        } else {
            false
        }
    }

    pub fn get_battery_protection(&self) -> Option<BatteryProtection> {
        if let Some(battery) = &self.battery
            && battery.has_protection()
        {
            Some(battery.get_protection_mode())
        } else {
            None
        }
    }

    pub fn set_battery_protection(&mut self, protection: BatteryProtection) {
        if let Some(battery) = &mut self.battery
            && battery.has_protection()
        {
            battery.set_protection_mode(protection);
        }
    }

    pub fn has_display_brightness(&self) -> bool {
        self.display_brightness.is_some()
    }

    pub fn get_display_brightness(&self) -> Option<Percentage> {
        self.display_brightness.as_ref().map(|o| o.get_brightness())
    }

    pub fn set_display_brightness(&mut self, brightness: Percentage) {
        if let Some(display_brightness) = &mut self.display_brightness {
            display_brightness.set_brightness(brightness);
        }
    }

    pub fn has_keyboard_backlight(&self) -> bool {
        self.keyboard_backlight.is_some()
    }

    pub fn get_keyboard_backlight(&self) -> Option<KeyboardBacklight> {
        self.keyboard_backlight.as_ref().map(|o| o.get_brightness())
    }

    pub fn set_keyboard_backlight(&mut self, backlight: KeyboardBacklight) {
        if let Some(keyboard_backlight) = &mut self.keyboard_backlight {
            keyboard_backlight.set_brightness(backlight);
        }
    }

    pub fn has_platform_profile(&self) -> bool {
        self.platform_profile.is_some()
    }

    pub fn get_platform_profile(&self) -> Option<PlatformProfile> {
        self.platform_profile.as_ref().map(|o| o.get_profile())
    }

    pub fn set_platform_profile(&self, profile: PlatformProfile) {
        if let Some(platform_profile) = &self.platform_profile {
            platform_profile.set_profile(profile);
        }
    }

    pub fn get_platform_profile_choices(&self) -> Option<PlatformProfileChoices> {
        self.platform_profile.as_ref().map(|o| o.get_profile_choices())
    }

    pub fn on_file_change(&mut self, server: &Server) {
        if let Some(tag) = self.file_listener.get_tag() {
            match tag {
                Tag::DisplayBrightness => {
                    if let Some(display_brightness) = &mut self.display_brightness {
                        server.broadcast(Message::DisplayBrightness(display_brightness.get_brightness()));
                    }
                }
                Tag::KeyboardBacklight => {
                    if let Some(keyboard_backlight) = &mut self.keyboard_backlight {
                        keyboard_backlight.start_polling();
                    }
                }
                Tag::PlatformProfile => {
                    if let Some(platform_profile) = &mut self.platform_profile {
                        server.broadcast(Message::PlatformProfile(platform_profile.get_profile()));
                    }
                }
            };
        }
    }

    pub fn poll_battery(&mut self, server: &Server) {
        if let Some(battery) = &mut self.battery {
            battery.poll(server);
        }
    }

    pub fn on_power_supply_event(&mut self) {
        if let Some(battery) = &mut self.battery {
            battery.on_power_supply_event();
        }
    }

    pub fn poll_keyboard_backlight(&mut self, server: &Server) {
        if let Some(keyboard_backlight) = &mut self.keyboard_backlight {
            keyboard_backlight.poll(server);
        }
    }

    pub fn get_state(&self) -> State {
        let (battery_protection, drop_threshold) = match &self.battery {
            Some(battery) => (battery.get_protection_mode(), battery.get_drop_threshold()),
            None => (BatteryProtection::default(), BatteryModule::DEFAULT_DROP_THRESHOLD),
        };
        let display_brightness = match &self.display_brightness {
            Some(display_brightness) => display_brightness.get_brightness(),
            None => Percentage::from(DisplayBrightnessModule::DEFAULT_DISPLAY_BRIGHTNESS),
        };
        let keyboard_backlight = match &self.keyboard_backlight {
            Some(keyboard_backlight) => keyboard_backlight.get_brightness(),
            None => KeyboardBacklight::default(),
        };
        let platform_profile = match &self.platform_profile {
            Some(platform_profile) => platform_profile.get_profile(),
            None => PlatformProfile::default(),
        };
        State {
            battery_protection,
            drop_threshold,
            display_brightness,
            keyboard_backlight,
            platform_profile,
        }
    }
}
