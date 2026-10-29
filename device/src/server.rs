use crate::{
    Event,
    device::Device,
    listeners::event::EventListener,
    wrappers::{Connection, File, FilePath, PathnameServer, ResourceID},
};
use device_common::{Get, Message, Request, SOCKET_PATH, Set, Signal};

pub const MAX_CLIENTS: usize = 10;

pub struct Server {
    pathname_server: PathnameServer,
    endpoints: Vec<Connection>,
}

impl Server {
    pub fn init() -> Self {
        Self {
            pathname_server: PathnameServer::new(&FilePath::from(SOCKET_PATH), MAX_CLIENTS),
            endpoints: Vec::with_capacity(MAX_CLIENTS),
        }
    }

    pub fn get_fd(&self) -> ResourceID {
        self.pathname_server.get_fd()
    }

    pub fn on_connection(&mut self, event_listener: &mut EventListener) {
        if let Some(endpoint) = self.pathname_server.accept_connection() {
            if self.endpoints.len() < MAX_CLIENTS {
                event_listener.add_event_source(endpoint.get_fd(), Event::Message(endpoint));
            } else {
                Self::send_response(endpoint, Message::Signal(Signal::MaxClients));
                endpoint.close();
            }
        }
    }

    pub fn on_message(&mut self, event_listener: &mut EventListener, device: &mut Device, endpoint: Connection) {
        if let Some(packet) = endpoint.receive() {
            match Request::try_from(packet) {
                Ok(request) => match request {
                    Request::Get(request) => self.get_request(device, endpoint, request),
                    Request::Set(request) => self.set_request(device, request),
                    Request::Subscribe => {
                        if device.has_battery() {
                            self.get_request(device, endpoint, Get::BatteryCharge);
                            self.get_request(device, endpoint, Get::BatteryStatus);
                            if device.has_battery_protection() {
                                self.get_request(device, endpoint, Get::BatteryProtection);
                            }
                        }
                        if device.has_display_brightness() {
                            self.get_request(device, endpoint, Get::DisplayBrightness);
                        }
                        if device.has_keyboard_backlight() {
                            self.get_request(device, endpoint, Get::KeyboardBacklight);
                        }
                        if device.has_platform_profile() {
                            self.get_request(device, endpoint, Get::PlatformProfile);
                            self.get_request(device, endpoint, Get::PlatformProfileChoices);
                        }
                        if !self.endpoints.contains(&endpoint) {
                            self.endpoints.push(endpoint);
                        }
                        Self::send_response(endpoint, Message::Signal(Signal::Synced));
                    }
                },
                Err(e) => {
                    println!("Error while parsing request: {}", e);
                    self.close_channel(event_listener, endpoint);
                }
            }
        } else {
            self.close_channel(event_listener, endpoint);
        }
    }

    fn get_request(&mut self, device: &mut Device, endpoint: Connection, request: Get) {
        let response = match request {
            Get::BatteryCharge => device.get_battery_charge().map(Message::BatteryCharge),
            Get::BatteryStatus => device.get_battery_status().map(Message::BatteryStatus),
            Get::BatteryProtection => device.get_battery_protection().map(Message::BatteryProtection),
            Get::DisplayBrightness => device.get_display_brightness().map(Message::DisplayBrightness),
            Get::KeyboardBacklight => device.get_keyboard_backlight().map(Message::KeyboardBacklight),
            Get::PlatformProfileChoices => device.get_platform_profile_choices().map(Message::PlatformProfileChoices),
            Get::PlatformProfile => device.get_platform_profile().map(Message::PlatformProfile),
        };
        match response {
            Some(message) => Self::send_response(endpoint, message),
            None => Self::send_response(endpoint, Message::Signal(Signal::Unavailable)),
        }
    }

    fn set_request(&mut self, device: &mut Device, request: Set) {
        match request {
            Set::BatteryProtection { protection } => {
                if device.has_battery_protection() {
                    device.set_battery_protection(protection);
                    self.broadcast(Message::BatteryProtection(protection));
                }
            }
            Set::DisplayBrightness { brightness } => device.set_display_brightness(brightness),
            Set::KeyboardBacklight { backlight } => device.set_keyboard_backlight(backlight),
            Set::PlatformProfile { profile } => device.set_platform_profile(profile),
        }
    }

    fn close_channel(&mut self, event_listener: &mut EventListener, endpoint: Connection) {
        event_listener.remove_event_source(endpoint.get_fd());
        self.endpoints.retain(|&e| e != endpoint);
        endpoint.close();
    }

    pub fn broadcast(&self, message: Message) {
        let packet = message.to_packet();
        self.endpoints.iter().for_each(|&endpoint| endpoint.send(&packet));
    }

    fn send_response(endpoint: Connection, message: Message) {
        endpoint.send(&message.to_packet());
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        File::delete(&FilePath::from(SOCKET_PATH));
    }
}
