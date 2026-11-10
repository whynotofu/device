use crate::{
    device::Listeners,
    file_var::{FileVariable, Mode},
    listeners::file::Tag,
    stack_string::StackString,
    state::State,
    wrappers::{File, FilePath},
};
use device_common::{PlatformProfile, PlatformProfileChoices, UNum};

type ProfileString = StackString<16>;
type ProfileChoicesString = StackString<128>;

pub struct PlatformProfileModule {
    profile_file: FileVariable<ProfileString>,
    profile_choices: PlatformProfileChoices,
}

impl PlatformProfileModule {
    pub fn new(state: &State, listeners: &mut Listeners) -> Option<PlatformProfileModule> {
        let profile_file = Self::profile_file();
        let profile_choices_file = Self::profile_choices_file();
        if File::exist(&profile_file) && File::exist(&profile_choices_file) {
            let profile_choices_string = FileVariable::<ProfileChoicesString>::static_get(&profile_choices_file);
            let profile_choices = PlatformProfileChoices::from(&profile_choices_string);

            if profile_choices.count() < 2 {
                return None;
            }

            let platform_profile = PlatformProfileModule {
                profile_file: FileVariable::<ProfileString>::new(&profile_file, Mode::ReadWrite),
                profile_choices,
            };
            if let Some(profile) = state.platform_profile {
                platform_profile.set_profile(profile);
            }
            listeners.file.add(&profile_file, Tag::PlatformProfile);
            Some(platform_profile)
        } else {
            None
        }
    }

    pub fn get_profile(&self) -> PlatformProfile {
        PlatformProfile::from(&self.profile_file.get())
    }

    pub fn set_profile(&self, profile: PlatformProfile) {
        if self.profile_choices.contains(profile) {
            self.profile_file.set(&ProfileString::from(profile.as_str()));
        }
    }

    pub fn get_profile_choices(&self) -> PlatformProfileChoices {
        self.profile_choices
    }

    fn profile_file() -> FilePath {
        Self::make_platform_profile_path("platform_profile")
    }

    fn profile_choices_file() -> FilePath {
        Self::make_platform_profile_path("platform_profile_choices")
    }

    fn make_platform_profile_path(file: &str) -> FilePath {
        FilePath::from("/sys/firmware/acpi/").add(file)
    }
}

impl From<&ProfileString> for PlatformProfile {
    fn from(profile: &ProfileString) -> Self {
        match profile.as_str() {
            "low-power" => PlatformProfile::LowPower,
            "cool" => PlatformProfile::Cool,
            "quiet" => PlatformProfile::Quiet,
            "balanced" => PlatformProfile::Balanced,
            "balanced-performance" => PlatformProfile::BalancedPerformance,
            "performance" => PlatformProfile::Performance,
            "max-power" => PlatformProfile::MaxPower,
            "custom" => PlatformProfile::Custom,
            _ => panic!("Unexpected platform profile string!"),
        }
    }
}

impl From<&ProfileChoicesString> for PlatformProfileChoices {
    fn from(choices: &ProfileChoicesString) -> Self {
        let mut list = 0 as UNum;
        for choice in choices.as_str().split(" ") {
            list |= (1 as UNum) << PlatformProfile::from(&ProfileString::from(choice)).index();
        }
        Self::from(list)
    }
}
