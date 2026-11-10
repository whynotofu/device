use crate::{
    file_var::{FileVariable, Mode},
    stack_string::StackString,
    wrappers::{Directory, File, FilePath, NetlinkSocket, ResourceID},
};
use std::collections::HashMap;

pub struct PowerSupply {
    online: bool,
}

pub struct NetlinkListener {
    netlink: NetlinkSocket,
    battery_name: Option<String>,
    power_supply_list: HashMap<String, PowerSupply>,
}

impl NetlinkListener {
    pub fn new() -> Self {
        Self {
            netlink: NetlinkSocket::new(),
            battery_name: None,
            power_supply_list: Self::find_power_supplies(),
        }
    }

    fn find_power_supplies() -> HashMap<String, PowerSupply> {
        let mut list = HashMap::new();
        let path = "/sys/class/power_supply";
        let directory = Directory::open(path);
        while let Some(name) = directory.get_entry() {
            let type_file = FilePath::from(path).add("/").add(name).add("/").add("type");
            if File::is_file(&type_file) {
                let power_supply_type = FileVariable::<StackString<16>>::new(&type_file, Mode::ReadOnly).get();
                let power_supply_type = power_supply_type.trim_end().into_lowercase();
                if ["mains", "usb", "usb_c", "usb_pd", "usb_pd_drp", "usb_dcp", "usb_aca"].contains(&power_supply_type.as_str())
                {
                    let online_file = FilePath::from(path).add("/").add(name).add("/").add("online");
                    if File::is_file(&online_file) {
                        let online = FileVariable::<u8>::new(&online_file, Mode::ReadOnly).get() == 1;
                        list.insert(name.to_string(), PowerSupply { online: online });
                    }
                }
            }
        }
        list
    }

    pub fn set_battery_name(&mut self, battery_name: Option<String>) {
        self.battery_name = battery_name;
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
                    if let Some(battery_name) = &self.battery_name
                        && name == battery_name
                    {
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
