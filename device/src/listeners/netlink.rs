use crate::wrappers::{NetlinkSocket, ResourceID};
use std::{
    collections::HashMap,
    {fs, path::Path},
};

pub struct PowerSupply {
    online: bool,
}

pub struct NetlinkListener {
    netlink: NetlinkSocket,
    battery_name: String,
    power_supply_list: HashMap<String, PowerSupply>,
}

impl NetlinkListener {
    pub fn new(battery_name: String) -> Self {
        Self {
            netlink: NetlinkSocket::new(),
            battery_name,
            power_supply_list: Self::find_power_supplies(),
        }
    }

    fn find_power_supplies() -> HashMap<String, PowerSupply> {
        let mut list = HashMap::new();
        let dir = Path::new("/sys/class/power_supply");
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let mut path = entry.path();
                if path.is_dir()
                    && let Some(name) = path.file_name().map(|os_str| os_str.to_string_lossy().into_owned())
                {
                    path.push("type");
                    if path.is_file()
                        && let Ok(power_supply_type) = fs::read_to_string(&path)
                        && ["mains", "usb", "usb_c", "usb_pd", "usb_pd_drp", "usb_dcp", "usb_aca"]
                            .contains(&power_supply_type.trim_end().to_ascii_lowercase().as_str())
                    {
                        path.pop();
                        path.push("online");
                        if path.is_file()
                            && let Ok(online) = fs::read_to_string(&path)
                        {
                            list.insert(
                                name.to_string(),
                                PowerSupply {
                                    online: online.trim_end() == "1",
                                },
                            );
                        }
                    }
                }
            }
        }
        list
    }

    pub fn possible_battery_status_change(&mut self) -> bool {
        let mut possible_change = false;
        let mut message = [0u8; 2048];
        let len = self.netlink.read(&mut message);
        let message = String::from_utf8_lossy(&message[..len]);
        let mut action = None;
        let mut subsystem = None;
        let mut devtype = None;
        let mut power_supply_type = None;
        let mut power_supply_name = None;
        let mut power_supply_online = None;
        for line in message.split('\0') {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "ACTION" => {
                        if value == "change" {
                            action = Some(Action::Change);
                        }
                    }
                    "SUBSYSTEM" => {
                        if value == "power_supply" {
                            subsystem = Some(SubSystem::PowerSupply);
                        }
                    }
                    "DEVTYPE" => {
                        if value == "power_supply" {
                            devtype = Some(DevType::PowerSupply);
                        }
                    }
                    "POWER_SUPPLY_TYPE" => match value {
                        "Mains" => power_supply_type = Some(PowerSupplyType::Mains),
                        "Battery" => power_supply_type = Some(PowerSupplyType::Battery),
                        _ => {}
                    },
                    "POWER_SUPPLY_NAME" => {
                        power_supply_name = Some(value);
                    }
                    "POWER_SUPPLY_ONLINE" => {
                        power_supply_online = Some(value == "1");
                    }
                    _ => {}
                }
            }
        }
        if action == Some(Action::Change)
            && subsystem == Some(SubSystem::PowerSupply)
            && devtype == Some(DevType::PowerSupply)
            && let Some(power_supply_type) = power_supply_type
            && let Some(name) = power_supply_name
        {
            match power_supply_type {
                PowerSupplyType::Mains => {
                    if let Some(online) = power_supply_online {
                        if let Some(power_supply) = self.power_supply_list.get_mut(name) {
                            if power_supply.online != online {
                                power_supply.online = online;
                                possible_change = true;
                            }
                        } else {
                            self.power_supply_list.insert(name.to_string(), PowerSupply { online });
                            possible_change = true;
                        }
                    }
                }
                PowerSupplyType::Battery => {
                    if name == self.battery_name {
                        possible_change = true;
                    }
                }
            }
        }
        possible_change
    }

    pub fn get_fd(&self) -> ResourceID {
        self.netlink.get_fd()
    }
}

#[derive(PartialEq)]
enum Action {
    Change,
}

#[derive(PartialEq)]
enum SubSystem {
    PowerSupply,
}

#[derive(PartialEq)]
enum DevType {
    PowerSupply,
}

#[derive(PartialEq)]
enum PowerSupplyType {
    Mains,
    Battery,
}
