use crate::wrappers::{KEventNetlinkSocket, ResourceID};
use std::{
    collections::HashMap,
    {fs, path::Path},
};

enum Match {
    Subsystem,
    PowerSupplyName,
    PowerSupplyOnline,
}

pub struct PowerSupply {
    online: bool,
}

pub struct PowerSupplyListener {
    kevent_netlink_socket: KEventNetlinkSocket,
    power_supply_list: HashMap<String, PowerSupply>,
}

impl PowerSupplyListener {
    pub fn new() -> Self {
        Self {
            kevent_netlink_socket: KEventNetlinkSocket::new(),
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

    pub fn online_status_changed(&mut self) -> bool {
        let mut bytes = [0u8; 2048];
        let len = self.kevent_netlink_socket.read(&mut bytes);
        let message = String::from_utf8_lossy(&bytes[..len]);
        let mut needed = Match::Subsystem;
        let mut name = "";
        let mut changed = false;
        for line in message.split('\0') {
            if let Some((key, value)) = line.split_once('=') {
                match needed {
                    Match::Subsystem => {
                        if key == "SUBSYSTEM" && value == "power_supply" {
                            needed = Match::PowerSupplyName;
                        }
                    }
                    Match::PowerSupplyName => {
                        if key == "POWER_SUPPLY_NAME" {
                            name = value;
                            needed = Match::PowerSupplyOnline;
                        }
                    }
                    Match::PowerSupplyOnline => {
                        if key == "POWER_SUPPLY_ONLINE" {
                            let (name, online) = (name.to_string(), value == "1");
                            if let Some(power_supply) = self.power_supply_list.get_mut(&name) {
                                if power_supply.online != online {
                                    power_supply.online = online;
                                    changed = true;
                                }
                            } else {
                                self.power_supply_list.insert(name, PowerSupply { online });
                                changed = true;
                            }
                            needed = Match::Subsystem;
                        }
                    }
                }
            }
        }
        changed
    }

    pub fn get_fd(&self) -> ResourceID {
        self.kevent_netlink_socket.get_fd()
    }
}
