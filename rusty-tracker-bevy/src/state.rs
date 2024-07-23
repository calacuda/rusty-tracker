use bevy::prelude::*;
use catppuccin::{Flavor, FlavorName, PALETTE};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
pub enum AppState {
    /// setup the window and stuff, is a transient state only entered once when the game is
    /// initially launched
    #[default]
    Setup,
    Main,
    Settings,
    ExitTracker,
}

#[derive(SubStates, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[source(AppState = AppState::Main)]
pub enum MainScreenMode {
    #[default]
    Move,
    Command,
    Edit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LightDarkMode {
    Light,
    #[default]
    Dark,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct StyleState {
    pub palette: Flavor,
    pub palette_name: FlavorName,
}

impl StyleState {
    pub fn light_dark_mode(&mut self, mode: LightDarkMode) {
        match mode {
            LightDarkMode::Light => self.set_palette(FlavorName::Latte),
            LightDarkMode::Dark => self.set_palette(FlavorName::Mocha),
        }
    }

    pub fn set_palette(&mut self, flavor: FlavorName) {
        match flavor {
            FlavorName::Latte => self.palette = PALETTE.latte,
            FlavorName::Frappe => self.palette = PALETTE.frappe,
            FlavorName::Macchiato => self.palette = PALETTE.macchiato,
            FlavorName::Mocha => self.palette = PALETTE.mocha,
        }

        self.palette_name = flavor;
    }
}

impl Default for StyleState {
    fn default() -> Self {
        Self {
            palette: PALETTE.mocha,
            palette_name: FlavorName::Mocha,
        }
    }
}
