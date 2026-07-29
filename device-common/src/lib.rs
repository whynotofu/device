use anyhow::{Error, Ok, Result, anyhow};
#[cfg(feature = "clap")]
use clap::{Subcommand, ValueEnum};
use enum_iterator::{Sequence, all};
#[cfg(feature = "clap")]
use std::str::FromStr;
use std::{
    fmt,
    ops::{Add, Sub},
};

pub const SOCKET_PATH: &str = "/tmp/device.sock";

pub type Packet = [u8; 2];

#[derive(Copy, Clone)]
#[cfg_attr(feature = "clap", derive(Subcommand))]
pub enum Request {
    #[cfg_attr(feature = "clap", command(subcommand))]
    Get(Get),
    #[cfg_attr(feature = "clap", command(subcommand))]
    Set(Set),
    #[cfg_attr(feature = "clap", command(skip))]
    Subscribe,
}

impl TryFrom<Packet> for Request {
    type Error = Error;

    fn try_from(bytes: Packet) -> Result<Self, Self::Error> {
        if bytes[0] < 255 {
            Set::try_from(bytes).map(Request::Set)
        } else {
            if bytes[1] < 255 {
                Get::try_from(bytes).map(Request::Get)
            } else {
                Ok(Request::Subscribe)
            }
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "clap", derive(Subcommand))]
pub enum Get {
    BatteryCharge,
    BatteryStatus,
    BatteryProtection,
    DisplayBrightness,
    KeyboardBacklight,
    PlatformProfile,
    PlatformProfileChoices,
}

impl TryFrom<Packet> for Get {
    type Error = Error;

    fn try_from(bytes: Packet) -> Result<Self, Self::Error> {
        match bytes[1] {
            1 => Ok(Get::BatteryCharge),
            2 => Ok(Get::BatteryStatus),
            3 => Ok(Get::BatteryProtection),
            4 => Ok(Get::DisplayBrightness),
            5 => Ok(Get::KeyboardBacklight),
            6 => Ok(Get::PlatformProfile),
            7 => Ok(Get::PlatformProfileChoices),
            _ => Err(anyhow!("Unknown get code")),
        }
    }
}

impl Get {
    pub fn to_packet(&self) -> [u8; 2] {
        let code = match self {
            Get::BatteryCharge => 1,
            Get::BatteryStatus => 2,
            Get::BatteryProtection => 3,
            Get::DisplayBrightness => 4,
            Get::KeyboardBacklight => 5,
            Get::PlatformProfile => 6,
            Get::PlatformProfileChoices => 7,
        };
        [255, code]
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "clap", derive(Subcommand))]
pub enum Set {
    BatteryProtection { protection: BatteryProtection },
    DisplayBrightness { brightness: Percentage },
    KeyboardBacklight { backlight: KeyboardBacklight },
    PlatformProfile { profile: PlatformProfile },
}

impl TryFrom<Packet> for Set {
    type Error = Error;

    fn try_from(bytes: Packet) -> Result<Self, Self::Error> {
        let (code, value) = (bytes[0], bytes[1]);
        match code {
            3 => Ok(Set::BatteryProtection {
                protection: BatteryProtection::try_from(value)?,
            }),
            4 => Ok(Set::DisplayBrightness {
                brightness: Percentage::from(value),
            }),
            5 => Ok(Set::KeyboardBacklight {
                backlight: KeyboardBacklight::from(value),
            }),
            6 => Ok(Set::PlatformProfile {
                profile: PlatformProfile::try_from(value)?,
            }),
            _ => Err(anyhow!("Unknown set code")),
        }
    }
}

impl Set {
    pub fn to_packet(&self) -> [u8; 2] {
        match self {
            Set::BatteryProtection { protection } => [3, protection.code()],
            Set::DisplayBrightness { brightness } => [4, brightness.value()],
            Set::KeyboardBacklight { backlight } => [5, backlight.value()],
            Set::PlatformProfile { profile } => [6, profile.code()],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Message {
    BatteryCharge(Percentage),
    BatteryStatus(BatteryStatus),
    BatteryProtection(BatteryProtection),
    DisplayBrightness(Percentage),
    KeyboardBacklight(KeyboardBacklight),
    PlatformProfile(PlatformProfile),
    PlatformProfileChoices(PlatformProfileChoices),
    Signal(Signal),
}

impl Message {
    pub fn to_packet(self) -> Packet {
        match self {
            Message::BatteryCharge(charge) => [1, charge.value()],
            Message::BatteryStatus(status) => [2, status.code()],
            Message::BatteryProtection(protection) => [3, protection.code()],
            Message::DisplayBrightness(brightness) => [4, brightness.value()],
            Message::KeyboardBacklight(backlight) => [5, backlight.value()],
            Message::PlatformProfile(profile) => [6, profile.code()],
            Message::PlatformProfileChoices(choices) => [7, choices.list()],
            Message::Signal(signal) => [255, signal.code()],
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Message::BatteryCharge(charge) => format!("battery_charge = {}", charge.value()),
            Message::BatteryStatus(status) => format!("battery_status = {}", status.as_str()),
            Message::BatteryProtection(protection) => format!("battery_protection = {}", protection.as_str()),
            Message::DisplayBrightness(brightness) => format!("display_brightness = {}", brightness),
            Message::KeyboardBacklight(backlight) => format!("keyboard_backlight = {}", backlight.as_str()),
            Message::PlatformProfile(profile) => format!("platform_profile = {}", profile.as_str()),
            Message::PlatformProfileChoices(choices) => format!("platform_profile_choices = {}", choices),
            Message::Signal(signal) => signal.as_str().to_string(),
        };
        write!(f, "{}", s)
    }
}

impl TryFrom<Packet> for Message {
    type Error = Error;

    fn try_from(bytes: Packet) -> Result<Self, Self::Error> {
        let (code, value) = (bytes[0], bytes[1]);
        match code {
            1 => Ok(Message::BatteryCharge(Percentage::from(value))),
            2 => BatteryStatus::try_from(value).map(Message::BatteryStatus),
            3 => BatteryProtection::try_from(value).map(Message::BatteryProtection),
            4 => Ok(Message::DisplayBrightness(Percentage::from(value))),
            5 => Ok(Message::KeyboardBacklight(KeyboardBacklight::from(value))),
            6 => PlatformProfile::try_from(value).map(Message::PlatformProfile),
            7 => Ok(Message::PlatformProfileChoices(PlatformProfileChoices::from(value))),
            255 => Signal::try_from(value).map(Message::Signal),
            _ => Err(anyhow!("Unknown message code")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Signal {
    Synced,
    Unavailable,
    MaxClients,
}

impl Signal {
    pub fn code(self) -> u8 {
        match self {
            Signal::Synced => 1,
            Signal::Unavailable => 2,
            Signal::MaxClients => 3,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Signal::Synced => "Synced",
            Signal::Unavailable => "Requested value is not available",
            Signal::MaxClients => "Server max clients limit exceeded",
        }
    }
}

impl TryFrom<u8> for Signal {
    type Error = Error;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            1 => Ok(Signal::Synced),
            2 => Ok(Signal::Unavailable),
            3 => Ok(Signal::MaxClients),
            _ => Err(anyhow!("Unknown signal code")),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Percentage {
    value: u8,
}

impl Add<u8> for Percentage {
    type Output = Self;

    fn add(mut self, value: u8) -> Self {
        self.value = self.value.saturating_add(value).min(100);
        self
    }
}

impl Sub<u8> for Percentage {
    type Output = Self;

    fn sub(mut self, value: u8) -> Self {
        self.value = self.value.saturating_sub(value);
        self
    }
}

impl Percentage {
    pub fn min(mut self, n: u8) -> Self {
        self.value = self.value.min(n);
        self
    }

    pub fn max(mut self, n: u8) -> Self {
        self.value = self.value.max(n.min(100));
        self
    }

    pub fn value(&self) -> u8 {
        self.value
    }
}

impl From<u8> for Percentage {
    fn from(value: u8) -> Self {
        let value = value.clamp(0, 100);
        Self { value }
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

#[cfg(feature = "clap")]
impl FromStr for Percentage {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s.parse::<u8>()?))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Sequence)]
pub enum BatteryStatus {
    Inactive,
    Charging,
    Discharging,
}

impl TryFrom<u8> for BatteryStatus {
    type Error = Error;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            1 => Ok(BatteryStatus::Inactive),
            2 => Ok(BatteryStatus::Charging),
            3 => Ok(BatteryStatus::Discharging),
            _ => Err(anyhow!("Unknown battery status code")),
        }
    }
}

impl BatteryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatteryStatus::Inactive => "inactive",
            BatteryStatus::Charging => "charging",
            BatteryStatus::Discharging => "discharging",
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            BatteryStatus::Inactive => 1,
            BatteryStatus::Charging => 2,
            BatteryStatus::Discharging => 3,
        }
    }
}

impl fmt::Display for BatteryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BatteryStatus::Inactive => "Inactive",
            BatteryStatus::Charging => "Charging",
            BatteryStatus::Discharging => "Discharging",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Sequence)]
#[cfg_attr(feature = "clap", derive(ValueEnum))]
pub enum BatteryProtection {
    Off,
    #[default]
    On,
    Stationary,
}

impl TryFrom<u8> for BatteryProtection {
    type Error = Error;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(BatteryProtection::Off),
            1 => Ok(BatteryProtection::On),
            2 => Ok(BatteryProtection::Stationary),
            _ => Err(anyhow!("Unknow battery protection code")),
        }
    }
}

impl BatteryProtection {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatteryProtection::Off => "off",
            BatteryProtection::On => "on",
            BatteryProtection::Stationary => "stationary",
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            BatteryProtection::Off => 0,
            BatteryProtection::On => 1,
            BatteryProtection::Stationary => 2,
        }
    }

    pub fn next(self) -> Option<Self> {
        Sequence::next(&self)
    }

    pub fn next_cyclic(self) -> Self {
        Sequence::next(&self).unwrap_or(Sequence::first().unwrap())
    }

    pub fn previous(self) -> Option<Self> {
        Sequence::previous(&self)
    }

    pub fn previous_cyclic(self) -> Self {
        Sequence::previous(&self).unwrap_or(Sequence::last().unwrap())
    }
}

impl fmt::Display for BatteryProtection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BatteryProtection::Off => "Off",
            BatteryProtection::On => "On",
            BatteryProtection::Stationary => "Stationary",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Sequence)]
#[cfg_attr(feature = "clap", derive(ValueEnum))]
pub enum KeyboardBacklight {
    #[default]
    Off,
    Low,
    Medium,
    Max,
}

impl KeyboardBacklight {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyboardBacklight::Off => "off",
            KeyboardBacklight::Low => "low",
            KeyboardBacklight::Medium => "medium",
            KeyboardBacklight::Max => "max",
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            KeyboardBacklight::Off => 0,
            KeyboardBacklight::Low => 1,
            KeyboardBacklight::Medium => 2,
            KeyboardBacklight::Max => 3,
        }
    }

    pub fn next(self) -> Option<Self> {
        Sequence::next(&self)
    }

    pub fn next_cyclic(self) -> Self {
        Sequence::next(&self).unwrap_or(Sequence::first().unwrap())
    }

    pub fn previous(self) -> Option<Self> {
        Sequence::previous(&self)
    }

    pub fn previous_cyclic(self) -> Self {
        Sequence::previous(&self).unwrap_or(Sequence::last().unwrap())
    }
}

impl From<u8> for KeyboardBacklight {
    fn from(value: u8) -> Self {
        match value {
            0 => KeyboardBacklight::Off,
            1 => KeyboardBacklight::Low,
            2 => KeyboardBacklight::Medium,
            _ => KeyboardBacklight::Max,
        }
    }
}

impl fmt::Display for KeyboardBacklight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            KeyboardBacklight::Off => "Off",
            KeyboardBacklight::Low => "Low",
            KeyboardBacklight::Medium => "Medium",
            KeyboardBacklight::Max => "Max",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PlatformProfileChoices {
    list: u8,
}

impl PlatformProfileChoices {
    pub fn first(&self) -> Option<PlatformProfile> {
        self.first_from(0, true, false)
    }

    pub fn last(&self) -> Option<PlatformProfile> {
        self.first_from(7, true, true)
    }

    pub fn next(&self, profile: PlatformProfile) -> Option<PlatformProfile> {
        self.first_from(profile.index(), false, false)
    }

    pub fn next_cyclic(&self, profile: PlatformProfile) -> Option<PlatformProfile> {
        let next = self.next(profile);
        if next.is_some() { next } else { self.first() }
    }

    pub fn previous(&self, profile: PlatformProfile) -> Option<PlatformProfile> {
        self.first_from(profile.index(), false, true)
    }

    pub fn previous_cyclic(&self, profile: PlatformProfile) -> Option<PlatformProfile> {
        let previous = self.previous(profile);
        if previous.is_some() { previous } else { self.last() }
    }

    fn first_from(&self, index: usize, inclusive: bool, reverse: bool) -> Option<PlatformProfile> {
        let step = if reverse { -1 } else { 1 };
        let mut index = if inclusive { index as i8 } else { index as i8 + step };
        while (0..=7).contains(&index) {
            if self.list & (1u8 << index) > 0 {
                return all::<PlatformProfile>().nth(index as usize);
            }
            index += step;
        }
        None
    }

    pub fn contains(&self, profile: PlatformProfile) -> bool {
        self.list & (1u8 << profile.index()) > 0
    }

    pub fn count(&self) -> usize {
        let mut count = 0;
        for i in 0..8 {
            if self.list & (1u8 << i) > 0 {
                count += 1;
            }
        }
        count
    }

    fn list(&self) -> u8 {
        self.list
    }
}

impl fmt::Display for PlatformProfileChoices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        write!(f, "[")?;
        for i in 0u8..8 {
            if self.list & (1u8 << i) > 0
                && let Result::Ok(profile) = PlatformProfile::try_from(i + 1)
            {
                if first {
                    first = false;
                } else {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", profile.as_str())?;
            }
        }
        write!(f, "]")
    }
}

impl From<u8> for PlatformProfileChoices {
    fn from(list: u8) -> Self {
        Self { list }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Sequence)]
#[cfg_attr(feature = "clap", derive(ValueEnum))]
pub enum PlatformProfile {
    LowPower,
    Cool,
    Quiet,
    #[default]
    Balanced,
    BalancedPerformance,
    Performance,
    MaxPower,
    Custom,
}

impl TryFrom<u8> for PlatformProfile {
    type Error = Error;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        if let Some(profile) = all::<PlatformProfile>().nth((code - 1) as usize) {
            Ok(profile)
        } else {
            Err(anyhow!("Unknown platform profile code"))
        }
    }
}

impl PlatformProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformProfile::LowPower => "low-power",
            PlatformProfile::Cool => "cool",
            PlatformProfile::Quiet => "quiet",
            PlatformProfile::Balanced => "balanced",
            PlatformProfile::BalancedPerformance => "balanced-performance",
            PlatformProfile::Performance => "performance",
            PlatformProfile::MaxPower => "max-power",
            PlatformProfile::Custom => "custom",
        }
    }

    pub fn index(&self) -> usize {
        all::<PlatformProfile>().position(|i| i == *self).unwrap()
    }

    pub fn code(&self) -> u8 {
        self.index() as u8 + 1
    }
}

impl fmt::Display for PlatformProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PlatformProfile::LowPower => "Low Power",
            PlatformProfile::Cool => "Cool",
            PlatformProfile::Quiet => "Quiet",
            PlatformProfile::Balanced => "Balanced",
            PlatformProfile::BalancedPerformance => "Balanced Performance",
            PlatformProfile::Performance => "Performance",
            PlatformProfile::MaxPower => "Max Power",
            PlatformProfile::Custom => "Custom",
        };
        write!(f, "{}", s)
    }
}
