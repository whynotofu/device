use anyhow::{Error, Ok, Result, anyhow};
#[cfg(feature = "clap")]
use clap::{Subcommand, ValueEnum};
use enum_iterator::{Sequence, all};
use std::{
    fmt,
    ops::{Add, Sub},
    str::FromStr,
};

pub const SOCKET_PATH: &str = "/tmp/device.sock";

pub type UNum = u16;

pub type Packet = [u8; 4];

fn get_packet_value(packet: Packet) -> UNum {
    UNum::from_le_bytes([packet[2], packet[3]])
}

fn pack(p_type: u8, code: u8, value: UNum) -> Packet {
    let value = value.to_le_bytes();
    [p_type, code, value[0], value[1]]
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "clap", derive(Subcommand))]
pub enum Request {
    #[cfg_attr(feature = "clap", command(subcommand))]
    Get(Get),
    #[cfg_attr(feature = "clap", command(subcommand))]
    Set(Set),
    #[cfg_attr(feature = "clap", command(skip))]
    MonitorMode,
}

impl TryFrom<Packet> for Request {
    type Error = Error;

    fn try_from(packet: Packet) -> Result<Self, Self::Error> {
        match packet[0] {
            1 => Get::try_from(packet).map(Request::Get),
            2 => Set::try_from(packet).map(Request::Set),
            3 => Ok(Request::MonitorMode),
            _ => Err(anyhow!("Unknown request code")),
        }
    }
}

impl Request {
    pub fn to_packet(&self) -> Packet {
        match self {
            Request::Get(get) => get.to_packet(),
            Request::Set(set) => set.to_packet(),
            Request::MonitorMode => pack(3, 0, 0),
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

    fn try_from(packet: Packet) -> Result<Self, Self::Error> {
        match packet[1] {
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
    pub fn to_packet(&self) -> Packet {
        let code = match self {
            Get::BatteryCharge => 1,
            Get::BatteryStatus => 2,
            Get::BatteryProtection => 3,
            Get::DisplayBrightness => 4,
            Get::KeyboardBacklight => 5,
            Get::PlatformProfile => 6,
            Get::PlatformProfileChoices => 7,
        };
        pack(1, code, 0)
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

    fn try_from(packet: Packet) -> Result<Self, Self::Error> {
        let value = get_packet_value(packet);

        match packet[1] {
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
    pub fn to_packet(&self) -> Packet {
        let (code, value) = match self {
            Set::BatteryProtection { protection } => (3, protection.code()),
            Set::DisplayBrightness { brightness } => (4, brightness.line_value()),
            Set::KeyboardBacklight { backlight } => (5, backlight.value()),
            Set::PlatformProfile { profile } => (6, profile.code()),
        };
        pack(2, code, value)
    }
}

#[derive(Copy, Clone)]
pub enum Response {
    Item(Item),
    Done,
    Unavailable,
    MaxClients,
}

impl TryFrom<Packet> for Response {
    type Error = Error;

    fn try_from(packet: Packet) -> Result<Self, Self::Error> {
        match packet[0] {
            1 => Item::try_from(packet).map(Response::Item),
            2 => Ok(Response::Done),
            3 => Ok(Response::Unavailable),
            4 => Ok(Response::MaxClients),
            _ => Err(anyhow!("Unknown response code")),
        }
    }
}

impl Response {
    pub fn to_packet(&self) -> Packet {
        match self {
            Response::Item(item) => item.to_packet(),
            Response::Done => pack(2, 0, 0),
            Response::Unavailable => pack(3, 0, 0),
            Response::MaxClients => pack(4, 0, 0),
        }
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Response::Item(item) => item.to_string(),
            Response::Done => "Done".to_string(),
            Response::Unavailable => "Requested value is not available".to_string(),
            Response::MaxClients => "Server max clients limit exceeded".to_string(),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Item {
    BatteryCharge(Percentage),
    BatteryStatus(BatteryStatus),
    BatteryProtection(BatteryProtection),
    DisplayBrightness(Percentage),
    KeyboardBacklight(KeyboardBacklight),
    PlatformProfile(PlatformProfile),
    PlatformProfileChoices(PlatformProfileChoices),
}

impl TryFrom<Packet> for Item {
    type Error = Error;

    fn try_from(packet: Packet) -> Result<Self, Self::Error> {
        let value = get_packet_value(packet);

        match packet[1] {
            1 => Ok(Item::BatteryCharge(Percentage::from(value))),
            2 => BatteryStatus::try_from(value).map(Item::BatteryStatus),
            3 => BatteryProtection::try_from(value).map(Item::BatteryProtection),
            4 => Ok(Item::DisplayBrightness(Percentage::from(value))),
            5 => Ok(Item::KeyboardBacklight(KeyboardBacklight::from(value))),
            6 => PlatformProfile::try_from(value).map(Item::PlatformProfile),
            7 => Ok(Item::PlatformProfileChoices(PlatformProfileChoices::from(value))),
            _ => Err(anyhow!("Unknown item code")),
        }
    }
}

impl Item {
    pub fn to_packet(self) -> Packet {
        let (code, value) = match self {
            Item::BatteryCharge(charge) => (1, charge.line_value()),
            Item::BatteryStatus(status) => (2, status.code()),
            Item::BatteryProtection(protection) => (3, protection.code()),
            Item::DisplayBrightness(brightness) => (4, brightness.line_value()),
            Item::KeyboardBacklight(backlight) => (5, backlight.value()),
            Item::PlatformProfile(profile) => (6, profile.code()),
            Item::PlatformProfileChoices(choices) => (7, choices.list()),
        };
        pack(1, code, value)
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Item::BatteryCharge(charge) => format!("battery_charge = {}", charge),
            Item::BatteryStatus(status) => format!("battery_status = {}", status.as_str()),
            Item::BatteryProtection(protection) => format!("battery_protection = {}", protection.as_str()),
            Item::DisplayBrightness(brightness) => format!("display_brightness = {}", brightness),
            Item::KeyboardBacklight(backlight) => format!("keyboard_backlight = {}", backlight.as_str()),
            Item::PlatformProfile(profile) => format!("platform_profile = {}", profile.as_str()),
            Item::PlatformProfileChoices(choices) => format!("platform_profile_choices = {}", choices),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Percentage {
    value: f32,
}

impl Add<f32> for Percentage {
    type Output = Self;

    fn add(mut self, value: f32) -> Self {
        self.value = self.value.add(value).clamp(0.0, Self::MAX);
        self
    }
}

impl Sub<f32> for Percentage {
    type Output = Self;

    fn sub(mut self, value: f32) -> Self {
        self.value = self.value.sub(value).clamp(0.0, Self::MAX);
        self
    }
}

impl Percentage {
    pub const MAX: f32 = 100.0;

    pub fn new(value: f32) -> Self {
        Self {
            value: (value.clamp(0.0, Self::MAX) * 100.0).round() / 100.0,
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn normalized(&self) -> f32 {
        self.value / Self::MAX
    }

    pub fn min(mut self, n: f32) -> Self {
        self.value = self.value.min(n).clamp(0.0, Self::MAX);
        self
    }

    pub fn max(mut self, n: f32) -> Self {
        self.value = self.value.max(n).clamp(0.0, Self::MAX);
        self
    }

    pub fn line_value(&self) -> UNum {
        (self.value * 100.0) as UNum
    }
}

impl From<UNum> for Percentage {
    fn from(value: UNum) -> Self {
        Self {
            value: value.clamp(0, 10000) as f32 / 100.0,
        }
    }
}

impl From<f32> for Percentage {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.value)
    }
}

impl FromStr for Percentage {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s.parse::<f32>()?))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Sequence)]
pub enum BatteryStatus {
    Inactive,
    Charging,
    Discharging,
}

impl TryFrom<UNum> for BatteryStatus {
    type Error = Error;

    fn try_from(code: UNum) -> Result<Self, Self::Error> {
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

    pub fn code(&self) -> UNum {
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

impl TryFrom<UNum> for BatteryProtection {
    type Error = Error;

    fn try_from(code: UNum) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(BatteryProtection::Off),
            1 => Ok(BatteryProtection::On),
            2 => Ok(BatteryProtection::Stationary),
            _ => Err(anyhow!("Unknow battery protection code")),
        }
    }
}

impl FromStr for BatteryProtection {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(BatteryProtection::Off),
            "on" => Ok(BatteryProtection::On),
            "stationary" => Ok(BatteryProtection::Stationary),
            _ => Err(anyhow!("Unknow battery protection string")),
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

    pub fn code(&self) -> UNum {
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

#[derive(Debug, Copy, Clone, PartialEq, Sequence)]
#[cfg_attr(feature = "clap", derive(ValueEnum))]
pub enum KeyboardBacklight {
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

    pub fn value(&self) -> UNum {
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

impl From<UNum> for KeyboardBacklight {
    fn from(value: UNum) -> Self {
        match value {
            0 => KeyboardBacklight::Off,
            1 => KeyboardBacklight::Low,
            2 => KeyboardBacklight::Medium,
            _ => KeyboardBacklight::Max,
        }
    }
}

impl FromStr for KeyboardBacklight {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(KeyboardBacklight::Off),
            "low" => Ok(KeyboardBacklight::Low),
            "medium" => Ok(KeyboardBacklight::Medium),
            "max" => Ok(KeyboardBacklight::Max),
            _ => Err(anyhow!("Unknow keyboard backlight string")),
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
    list: UNum,
}

impl PlatformProfileChoices {
    pub fn first(&self) -> Option<PlatformProfile> {
        self.first_from(0, true, false)
    }

    pub fn last(&self) -> Option<PlatformProfile> {
        self.first_from(PlatformProfile::CARDINALITY - 1, true, true)
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
        while (0..(PlatformProfile::CARDINALITY as i8)).contains(&index) {
            if self.list & ((1 as UNum) << index) > 0 {
                return all::<PlatformProfile>().nth(index as usize);
            }
            index += step;
        }
        None
    }

    pub fn contains(&self, profile: PlatformProfile) -> bool {
        self.list & ((1 as UNum) << profile.index()) > 0
    }

    pub fn count(&self) -> usize {
        let mut count = 0;
        for i in 0..PlatformProfile::CARDINALITY {
            if self.list & ((1 as UNum) << i) > 0 {
                count += 1;
            }
        }
        count
    }

    fn list(&self) -> UNum {
        self.list
    }
}

impl fmt::Display for PlatformProfileChoices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        write!(f, "[")?;
        for i in 0..PlatformProfile::CARDINALITY {
            if self.list & ((1 as UNum) << i) > 0
                && let Result::Ok(profile) = PlatformProfile::try_from((i as UNum) + 1)
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

impl From<UNum> for PlatformProfileChoices {
    fn from(list: UNum) -> Self {
        Self { list }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Sequence)]
#[cfg_attr(feature = "clap", derive(ValueEnum))]
pub enum PlatformProfile {
    LowPower,
    Cool,
    Quiet,
    Balanced,
    BalancedPerformance,
    Performance,
    MaxPower,
    Custom,
}

impl TryFrom<UNum> for PlatformProfile {
    type Error = Error;

    fn try_from(code: UNum) -> Result<Self, Self::Error> {
        if let Some(profile) = all::<PlatformProfile>().nth((code - 1) as usize) {
            Ok(profile)
        } else {
            Err(anyhow!("Unknown platform profile code"))
        }
    }
}

impl FromStr for PlatformProfile {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low-power" => Ok(PlatformProfile::LowPower),
            "cool" => Ok(PlatformProfile::Cool),
            "quiet" => Ok(PlatformProfile::Quiet),
            "balanced" => Ok(PlatformProfile::Balanced),
            "balanced-performance" => Ok(PlatformProfile::BalancedPerformance),
            "performance" => Ok(PlatformProfile::Performance),
            "max-power" => Ok(PlatformProfile::MaxPower),
            "custom" => Ok(PlatformProfile::Custom),
            _ => Err(anyhow!("Unknow platform profile string")),
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

    pub fn code(&self) -> UNum {
        self.index() as UNum + 1
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
