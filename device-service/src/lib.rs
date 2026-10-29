use anyhow::{Context, Result};
pub use device_common::{
    BatteryProtection, BatteryStatus, KeyboardBacklight, Percentage, PlatformProfile, PlatformProfileChoices, Signal,
};
use device_common::{Packet, SOCKET_PATH, Set};
use log::warn;
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixStream, unix::OwnedWriteHalf},
    sync::{Mutex, mpsc},
};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Default)]
pub struct DeviceService {
    battery_charge: Option<Percentage>,
    battery_status: Option<BatteryStatus>,
    battery_protection: Option<BatteryProtection>,
    display_brightness: Option<Percentage>,
    keyboard_backlight: Option<KeyboardBacklight>,
    platform_profile: Option<PlatformProfile>,
    platform_profile_choices: Option<PlatformProfileChoices>,
    writer: Option<Arc<Mutex<OwnedWriteHalf>>>,
}

type Update = device_common::Message;

#[derive(Debug, Clone)]
pub enum Message {
    Update(Update),
    SetWriter(Arc<Mutex<OwnedWriteHalf>>),
    Reset,
}

impl DeviceService {
    pub fn get_battery_info(&self) -> Option<(Percentage, BatteryStatus)> {
        if let (Some(charge), Some(status)) = (self.battery_charge, self.battery_status) {
            Some((charge, status))
        } else {
            None
        }
    }

    pub fn get_battery_protection(&self) -> Option<BatteryProtection> {
        self.battery_protection
    }

    pub fn set_battery_protection(&mut self, protection: BatteryProtection) {
        self.set_request(Set::BatteryProtection { protection });
    }

    pub fn get_display_brightness(&self) -> Option<Percentage> {
        self.display_brightness
    }

    pub fn set_display_brightness(&mut self, brightness: Percentage) {
        self.set_request(Set::DisplayBrightness { brightness });
    }

    pub fn get_keyboard_backlight(&self) -> Option<KeyboardBacklight> {
        self.keyboard_backlight
    }

    pub fn set_keyboard_backlight(&mut self, backlight: KeyboardBacklight) {
        self.set_request(Set::KeyboardBacklight { backlight })
    }

    pub fn get_platform_profile(&self) -> Option<PlatformProfile> {
        self.platform_profile
    }

    pub fn set_platform_profile(&mut self, profile: PlatformProfile) {
        self.set_request(Set::PlatformProfile { profile });
    }

    pub fn get_platform_profile_package(&self) -> Option<(PlatformProfileChoices, PlatformProfile)> {
        if let (Some(choices), Some(profile)) = (self.platform_profile_choices, self.platform_profile) {
            Some((choices, profile))
        } else {
            None
        }
    }

    fn set_request(&self, set: Set) {
        if let Some(writer) = self.writer.clone() {
            tokio::spawn(async move {
                if let Err(e) = writer.lock().await.write_all(&set.to_packet()).await {
                    warn!("DeviceService.set_request: {}", e);
                }
            });
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update(update) => match update {
                Update::BatteryCharge(charge) => self.battery_charge = Some(charge),
                Update::BatteryStatus(status) => self.battery_status = Some(status),
                Update::BatteryProtection(protection) => {
                    self.battery_protection = Some(protection);
                }
                Update::DisplayBrightness(brightness) => {
                    self.display_brightness = Some(brightness);
                }
                Update::KeyboardBacklight(backlight) => {
                    self.keyboard_backlight = Some(backlight);
                }
                Update::PlatformProfile(profile) => {
                    self.platform_profile = Some(profile);
                }
                Update::PlatformProfileChoices(choices) => {
                    self.platform_profile_choices = Some(choices);
                }
                Update::Signal(signal) => warn!("{}", signal.as_str()),
            },
            Message::SetWriter(writer) => self.writer = Some(writer),
            Message::Reset => {
                self.battery_charge = None;
                self.battery_status = None;
                self.battery_protection = None;
                self.display_brightness = None;
                self.keyboard_backlight = None;
                self.platform_profile = None;
                self.platform_profile_choices = None;
                self.writer = None;
            }
        }
    }

    pub fn run() -> ReceiverStream<Message> {
        let (output, receiver) = mpsc::channel::<Message>(100);

        let mut output = output.clone();

        tokio::spawn(async move {
            let intervals = [2u16, 5, 10, 15, 30, 60, 600];
            let mut attempt = 1;

            loop {
                if let Err(e) = Self::feed(&mut output, &mut attempt).await {
                    warn!("DeviceService: {}", e);
                }
                output.send(Message::Reset).await.expect("Failed to reset");

                if attempt <= intervals.len() {
                    tokio::time::sleep(Duration::from_secs(intervals[attempt - 1] as u64)).await;
                    attempt += 1;
                } else {
                    break;
                }
            }
        });

        ReceiverStream::new(receiver)
    }

    async fn feed(output: &mut mpsc::Sender<Message>, atempt: &mut usize) -> Result<()> {
        let Ok(stream) = UnixStream::connect(SOCKET_PATH).await else {
            return Ok(());
        };

        let (mut reader, mut writer) = stream.into_split();
        let mut packet = Packet::default();

        writer.write_all(&[255, 255]).await.context("socket closed")?;

        let writer = Arc::new(Mutex::new(writer));

        loop {
            reader.read_exact(&mut packet).await.context("socket closed")?;

            match Update::try_from(packet) {
                Ok(update) => {
                    if update == Update::Signal(Signal::Synced) {
                        output.send(Message::SetWriter(Arc::clone(&writer))).await?;
                        *atempt = 1;
                    } else {
                        output.send(Message::Update(update)).await?;
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}
