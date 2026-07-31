#[cfg(not(target_os = "linux"))]
compile_error!("This program only supports Linux");

use crate::{
    config::Config,
    device::Device,
    listeners::event::{Event, EventListener},
    server::Server,
    state::StateManager,
    wrappers::{process_signal, running_as_root, setup_signal_handler},
};

mod config;
mod device;
mod file_var;
mod listeners;
mod modules;
mod server;
mod stack_string;
mod state;
mod timer;
mod wrappers;

fn main() {
    if !running_as_root() {
        eprintln!("This process requires root privilege!");
        std::process::exit(1);
    }

    let state_manager = StateManager::init();
    let config = Config::init();
    config.get_static_config().map(StateManager::apply_static_config);
    let mut event_listener = EventListener::init(32);
    let mut device = Device::init(&config, &mut event_listener, &state_manager.get_state_on_file());
    let mut server = Server::init();
    let signal_fd = setup_signal_handler();

    event_listener.add_event_source(server.get_fd(), Event::Connection);
    event_listener.add_event_source(signal_fd, Event::Signal);

    let mut running = true;

    while running {
        match event_listener.event() {
            Event::Connection => server.on_connection(&mut event_listener),
            Event::Message(endpoint) => server.on_message(&mut event_listener, &mut device, endpoint),
            Event::FileChange => device.on_file_change(&server),
            Event::BatteryPoll => device.poll_battery(&server),
            Event::PowerSupply => device.on_power_supply_event(),
            Event::KeyboardBacklightPoll => device.poll_keyboard_backlight(&server),
            Event::Signal => {
                running = process_signal(signal_fd);
            }
        };
    }

    state_manager.sync(&device.get_state());
}
