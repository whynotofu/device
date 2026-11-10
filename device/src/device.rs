use crate::{
    config::Config,
    listeners::{
        event::{Event, EventListener},
        file::{FileListener, Tag},
        netlink::NetlinkListener,
    },
    modules::{
        battery::BatteryModule, display_brightness::DisplayBrightnessModule, keyboard_backlight::KeyboardBacklightModule,
        platform_profile::PlatformProfileModule,
    },
    server::Server,
    state::State,
};
use device_common::{
    BatteryProtection, BatteryStatus, Item, KeyboardBacklight, Percentage, PlatformProfile, PlatformProfileChoices,
};

pub struct Listeners {
    pub event: EventListener,
    pub file: FileListener,
    pub netlink: NetlinkListener,
}

pub struct Device {
    battery: Option<BatteryModule>,
    display_brightness: Option<DisplayBrightnessModule>,
    keyboard_backlight: Option<KeyboardBacklightModule>,
    platform_profile: Option<PlatformProfileModule>,
    pub listeners: Listeners,
}

impl Device {
    pub fn init(config: &Config, state: &State) -> Self {
        let mut listeners = Listeners {
            event: EventListener::init(32),
            file: FileListener::init(4),
            netlink: NetlinkListener::new(),
        };

        let battery = BatteryModule::new(config, state, &mut listeners);
        let display_brightness = DisplayBrightnessModule::new(state, &mut listeners);
        let keyboard_backlight = KeyboardBacklightModule::new(config, state, &mut listeners);
        let platform_profile = PlatformProfileModule::new(state, &mut listeners);

        if battery.is_none() && display_brightness.is_none() && keyboard_backlight.is_none() && platform_profile.is_none() {
            eprintln!("No modules initialized!");
            std::process::exit(0);
        }

        listeners.event.add(listeners.file.get_fd(), Event::FileChange);
        listeners.event.add(listeners.netlink.get_fd(), Event::Netlink);

        Self {
            battery,
            display_brightness,
            keyboard_backlight,
            platform_profile,
            listeners,
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
        if let Some(tag) = self.listeners.file.get_tag() {
            match tag {
                Tag::DisplayBrightness => {
                    if let Some(display_brightness) = &mut self.display_brightness {
                        server.broadcast(Item::DisplayBrightness(display_brightness.get_brightness()));
                    }
                }
                Tag::KeyboardBacklight => {
                    if let Some(keyboard_backlight) = &mut self.keyboard_backlight {
                        keyboard_backlight.start_polling();
                    }
                }
                Tag::PlatformProfile => {
                    if let Some(platform_profile) = &mut self.platform_profile {
                        server.broadcast(Item::PlatformProfile(platform_profile.get_profile()));
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

    pub fn on_netlink_event(&mut self) {
        if self.listeners.netlink.possible_battery_status_change()
            && let Some(battery) = &mut self.battery
        {
            battery.start_fast_poller();
        }
    }

    pub fn poll_keyboard_backlight(&mut self, server: &Server) {
        if let Some(keyboard_backlight) = &mut self.keyboard_backlight {
            keyboard_backlight.poll(server);
        }
    }

    pub fn get_state(&self) -> State {
        State {
            battery_protection: self.battery.as_ref().map(|battery| battery.get_protection_mode()),
            drop_threshold: self.battery.as_ref().map(|battery| battery.get_drop_threshold()),
            display_brightness: self.display_brightness.as_ref().map(|display_brightness| display_brightness.get_brightness()),
            keyboard_backlight: self.keyboard_backlight.as_ref().map(|keyboard_backlight| keyboard_backlight.get_brightness()),
            platform_profile: self.platform_profile.as_ref().map(|platform_profile| platform_profile.get_profile()),
        }
    }
}
