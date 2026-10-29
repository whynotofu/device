use crate::{
    file_var::{FileVariable, Mode},
    listeners::netlink::NetlinkListener,
    server::Server,
    stack_string::StackString,
    timer::Timer,
    wrappers::{File, FilePath, ResourceID},
};
use device_common::{BatteryProtection, BatteryStatus, Message, Percentage};
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
    netlink_listener: NetlinkListener,
    polling_interval: u8,
    poller: Timer,
}

impl BatteryModule {
    pub const DEFAULT_DROP_THRESHOLD: bool = true;

    pub fn new(polling_interval: u8, protection_mode: BatteryProtection, drop_threshold: bool) -> Option<BatteryModule> {
        Self::find().map(|name| Self::init(name, polling_interval, protection_mode, drop_threshold))
    }

    fn init(name: &str, polling_interval: u8, protection_mode: BatteryProtection, drop_threshold: bool) -> BatteryModule {
        let charge_file = FileVariable::<u8>::new(&Self::charge_file(name), Mode::ReadOnly);
        let charge = charge_file.get();
        let status_file = FileVariable::<StatusString>::new(&Self::status_file(name), Mode::ReadOnly);
        let status = BatteryStatus::from(status_file.get());
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
            netlink_listener: NetlinkListener::new(name.to_string()),
            polling_interval,
            poller: Timer::new(),
        };

        battery.set_protection_mode(protection_mode);
        battery.start_default_poller();
        println!("Battery: {}", name);
        battery
    }

    fn start_default_poller(&mut self) {
        self.poller.set(Duration::from_secs(self.polling_interval as u64), None).start();
    }

    fn start_millisecond_poller(&mut self) {
        self.poller.set(Duration::from_millis(250), Some(10)).start();
    }

    pub fn get_timer_fd(&self) -> ResourceID {
        self.poller.get_fd()
    }

    pub fn get_netlink_fd(&self) -> ResourceID {
        self.netlink_listener.get_fd()
    }

    pub fn get_charge(&self) -> Percentage {
        Percentage::from(self.charge)
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
            self.start_millisecond_poller();
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
            server.broadcast(Message::BatteryCharge(Percentage::from(charge)));
        }
        if self.status != status {
            self.status = status;
            server.broadcast(Message::BatteryStatus(status));
        }
    }

    pub fn on_netlink_event(&mut self) {
        if self.netlink_listener.possible_battery_status_change() {
            self.start_millisecond_poller();
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
