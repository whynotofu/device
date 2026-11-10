use crate::{
    Event,
    device::Device,
    listeners::event::EventListener,
    wrappers::{Connection, File, FilePath, PathnameServer, ResourceID},
};
use device_common::{Get, Item, Request, Response, SOCKET_PATH, Set};

pub const MAX_CLIENTS: usize = 10;

pub struct Server {
    pathname_server: PathnameServer,
    monitors: Vec<Connection>,
}

impl Server {
    pub fn init() -> Self {
        Self {
            pathname_server: PathnameServer::new(&FilePath::from(SOCKET_PATH), MAX_CLIENTS),
            monitors: Vec::with_capacity(MAX_CLIENTS),
        }
    }

    pub fn get_fd(&self) -> ResourceID {
        self.pathname_server.get_fd()
    }

    pub fn on_connection(&mut self, device: &mut Device) {
        if let Some(endpoint) = self.pathname_server.accept_connection() {
            if self.monitors.len() < MAX_CLIENTS {
                device.listeners.event.add(endpoint.get_fd(), Event::Message(endpoint));
            } else {
                Self::send_response(endpoint, Response::MaxClients);
                endpoint.close();
            }
        }
    }

    pub fn on_message(&mut self, device: &mut Device, endpoint: Connection) {
        if let Some(packet) = endpoint.receive() {
            match Request::try_from(packet) {
                Ok(request) => match request {
                    Request::Get(request) => Self::get_request(device, endpoint, request),
                    Request::Set(request) => self.set_request(device, request),
                    Request::MonitorMode => {
                        Self::send_response(endpoint, Response::Done);
                        if device.has_battery() {
                            Self::send_item(device, endpoint, Get::BatteryCharge);
                            Self::send_item(device, endpoint, Get::BatteryStatus);
                            if device.has_battery_protection() {
                                Self::send_item(device, endpoint, Get::BatteryProtection);
                            }
                        }
                        if device.has_display_brightness() {
                            Self::send_item(device, endpoint, Get::DisplayBrightness);
                        }
                        if device.has_keyboard_backlight() {
                            Self::send_item(device, endpoint, Get::KeyboardBacklight);
                        }
                        if device.has_platform_profile() {
                            Self::send_item(device, endpoint, Get::PlatformProfile);
                            Self::send_item(device, endpoint, Get::PlatformProfileChoices);
                        }
                        if !self.monitors.contains(&endpoint) {
                            self.monitors.push(endpoint);
                        }
                    }
                },
                Err(e) => {
                    println!("Error while parsing request: {}", e);
                    self.close_channel(&mut device.listeners.event, endpoint);
                }
            }
        } else {
            self.close_channel(&mut device.listeners.event, endpoint);
        }
    }

    fn get_item(device: &mut Device, get: Get) -> Option<Item> {
        match get {
            Get::BatteryCharge => device.get_battery_charge().map(Item::BatteryCharge),
            Get::BatteryStatus => device.get_battery_status().map(Item::BatteryStatus),
            Get::BatteryProtection => device.get_battery_protection().map(Item::BatteryProtection),
            Get::DisplayBrightness => device.get_display_brightness().map(Item::DisplayBrightness),
            Get::KeyboardBacklight => device.get_keyboard_backlight().map(Item::KeyboardBacklight),
            Get::PlatformProfileChoices => device.get_platform_profile_choices().map(Item::PlatformProfileChoices),
            Get::PlatformProfile => device.get_platform_profile().map(Item::PlatformProfile),
        }
    }

    fn get_request(device: &mut Device, endpoint: Connection, request: Get) {
        if let Some(item) = Self::get_item(device, request) {
            Self::send_response(endpoint, Response::Item(item));
        } else {
            Self::send_response(endpoint, Response::Unavailable);
        }
    }

    fn send_response(endpoint: Connection, response: Response) {
        endpoint.send(&response.to_packet());
    }

    fn set_request(&mut self, device: &mut Device, request: Set) {
        match request {
            Set::BatteryProtection { protection } => {
                if device.has_battery_protection() {
                    device.set_battery_protection(protection);
                    self.broadcast(Item::BatteryProtection(protection));
                }
            }
            Set::DisplayBrightness { brightness } => device.set_display_brightness(brightness),
            Set::KeyboardBacklight { backlight } => device.set_keyboard_backlight(backlight),
            Set::PlatformProfile { profile } => device.set_platform_profile(profile),
        }
    }

    fn send_item(device: &mut Device, endpoint: Connection, item: Get) {
        if let Some(item) = Self::get_item(device, item) {
            endpoint.send(&item.to_packet());
        }
    }

    fn close_channel(&mut self, event_listener: &mut EventListener, endpoint: Connection) {
        event_listener.remove(endpoint.get_fd());
        self.monitors.retain(|&e| e != endpoint);
        endpoint.close();
    }

    pub fn broadcast(&self, message: Item) {
        let packet = message.to_packet();
        self.monitors.iter().for_each(|&endpoint| endpoint.send(&packet));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        File::delete(&FilePath::from(SOCKET_PATH));
    }
}
