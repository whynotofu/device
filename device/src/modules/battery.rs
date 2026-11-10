use crate::{
    config::Config,
    device::Listeners,
    file_var::{FileVariable, Mode},
    listeners::event::Event,
    server::Server,
    stack_string::StackString,
    state::State,
    timer::Timer,
    wrappers::{File, FilePath},
};
use device_common::{BatteryProtection, BatteryStatus, Item, Percentage};
use std::{
    ops::{Add, Sub},
    time::Duration,
};

const BATTERY_DEVS: [&str; 3] = ["BAT0", "BAT1", "BATT"];

pub type StatusString = StackString<16>;

pub struct BatteryModule {
    charge_file: FileVariable<u8>,
    charge: u8,
    status_file: FileVariable<StatusString>,
    status: BatteryStatus,
    protection_mode: BatteryProtection,
    start_threshold_file: Option<FileVariable<u8>>,
    end_threshold_file: Option<FileVariable<u8>>,
    drop_threshold: bool,
    polling_interval: u8,
    poller: Timer,
}

impl BatteryModule {
    pub fn new(config: &Config, state: &State, listeners: &mut Listeners) -> Option<BatteryModule> {
        Self::find().map(|name| Self::init(name, config, state, listeners))
    }

    fn init(name: &str, config: &Config, state: &State, listeners: &mut Listeners) -> BatteryModule {
        let charge_file = FileVariable::<u8>::new(&Self::charge_file(name), Mode::ReadOnly);
        let charge = charge_file.get();
        let status_file = FileVariable::<StatusString>::new(&Self::status_file(name), Mode::ReadOnly);
        let status = BatteryStatus::from(status_file.get());
        let protection_mode = state.battery_protection.unwrap_or_default();
        let drop_threshold = state.drop_threshold.unwrap_or(true);
        let start_threshold_file = if File::exist(&Self::start_threshold_file(name)) {
            Some(FileVariable::<u8>::new(&Self::start_threshold_file(name), Mode::ReadWrite))
        } else {
            None
        };
        let end_threshold_file = if File::exist(&Self::end_threshold_file(name)) {
            Some(FileVariable::<u8>::new(&Self::end_threshold_file(name), Mode::ReadWrite))
        } else {
            None
        };

        let mut battery = BatteryModule {
            charge_file,
            charge,
            status_file,
            status,
            protection_mode,
            start_threshold_file,
            end_threshold_file,
            drop_threshold,
            polling_interval: config.battery_polling_interval,
            poller: Timer::new(),
        };

        battery.set_protection_mode(protection_mode);
        listeners.netlink.set_battery_name(Some(name.to_string()));
        listeners.event.add(battery.poller.get_fd(), Event::BatteryPoll);
        battery.start_default_poller();
        println!("Battery: {}", name);
        battery
    }

    fn start_default_poller(&mut self) {
        self.poller.set(Duration::from_secs(self.polling_interval as u64), None).start();
    }

    pub fn start_fast_poller(&mut self) {
        self.poller.set(Duration::from_millis(250), Some(10)).start();
    }

    pub fn get_charge(&self) -> Percentage {
        Percentage::from(self.charge as f32)
    }

    pub fn get_status(&self) -> BatteryStatus {
        self.status
    }

    pub fn has_protection(&self) -> bool {
        self.end_threshold_file.is_some()
    }

    pub fn get_protection_mode(&self) -> BatteryProtection {
        self.protection_mode
    }

    pub fn get_drop_threshold(&self) -> bool {
        self.drop_threshold
    }

    pub fn set_protection_mode(&mut self, mode: BatteryProtection) {
        self.protection_mode = mode;
        if let Some(end_threshold_file) = &self.end_threshold_file {
            let (start_threshold, end_threshold) = Self::get_thresholds(mode);
            if let Some(start_threshold_file) = &self.start_threshold_file {
                start_threshold_file.set(&start_threshold);
                end_threshold_file.set(&end_threshold);
            } else {
                self.drop_threshold = true;
                end_threshold_file.set(&start_threshold.sub(5));
            }
            self.start_fast_poller();
        }
    }

    pub fn poll(&mut self, server: &Server) {
        self.poller.update();
        if !self.poller.running() {
            self.start_default_poller();
        }
        let charge = self.charge_file.get();
        let status = BatteryStatus::from(self.status_file.get());
        if let Some(end_threshold_file) = &self.end_threshold_file
            && self.start_threshold_file.is_none()
        {
            let (start_threshold, end_threshold) = Self::get_thresholds(self.protection_mode);
            if self.drop_threshold {
                if charge <= start_threshold {
                    self.drop_threshold = false;
                    end_threshold_file.set(&end_threshold.add(5).min(100));
                }
            } else {
                if charge >= end_threshold {
                    self.drop_threshold = true;
                    end_threshold_file.set(&start_threshold.sub(5));
                }
            }
        }
        if self.charge != charge {
            self.charge = charge;
            server.broadcast(Item::BatteryCharge(Percentage::from(charge as f32)));
        }
        if self.status != status {
            self.status = status;
            server.broadcast(Item::BatteryStatus(status));
        }
    }

    fn get_thresholds(mode: BatteryProtection) -> (u8, u8) {
        match mode {
            BatteryProtection::Off => (95, 100),
            BatteryProtection::On => (75, 80),
            BatteryProtection::Stationary => (40, 60),
        }
    }

    fn is_valid(name: &str) -> bool {
        File::exist(&Self::charge_file(name)) && File::exist(&Self::status_file(name))
    }

    fn find() -> Option<&'static str> {
        BATTERY_DEVS.into_iter().find(|&name| Self::is_valid(name))
    }

    fn charge_file(name: &str) -> FilePath {
        Self::make_battery_path(name, "capacity")
    }

    fn status_file(name: &str) -> FilePath {
        Self::make_battery_path(name, "status")
    }

    fn start_threshold_file(name: &str) -> FilePath {
        Self::make_battery_path(name, "charge_control_start_threshold")
    }

    fn end_threshold_file(name: &str) -> FilePath {
        Self::make_battery_path(name, "charge_control_end_threshold")
    }

    fn make_battery_path(name: &str, file: &str) -> FilePath {
        FilePath::from("/sys/class/power_supply/").add(name).add("/").add(file)
    }
}

impl From<StatusString> for BatteryStatus {
    fn from(s: StatusString) -> Self {
        match s.into_lowercase().as_str() {
            "not charging" | "full" | "unknown" => BatteryStatus::Inactive,
            "charging" => BatteryStatus::Charging,
            "discharging" => BatteryStatus::Discharging,
            _ => panic!("Unexpected battery status string!"),
        }
    }
}
