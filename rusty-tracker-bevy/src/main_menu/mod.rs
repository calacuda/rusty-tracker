use crate::state::{AppState, MainScreenMode};
use bevy::prelude::*;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CursorLocation::default())
            // .insert_resource(CursorLocation::default())
            .add_systems(OnEnter(AppState::Main), init_for_main)
            // .add_systems(Startup, setup_camera)
            // .insert_resource(StyleState::default())
            // .add_systems(OnExit(AppState::Setup), setup_camera)
            // .add_systems(Update, make_visible.run_if(in_state(AppState::Setup)));
            .add_systems(Update, update_cursor.run_if(in_state(MainScreenMode::Move)));
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CursorLocation(Location);

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Location {
    pub row: usize,
    pub col: usize,
}

impl Location {
    pub fn row(self) -> usize {
        self.row
    }

    pub fn channel(self) -> usize {
        self.col / 6
    }

    pub fn note_num(self) -> usize {
        self.col % 6
    }
}

fn init_for_main() {}

fn update_cursor(
    mut cursor_loc: ResMut<CursorLocation>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_released(KeyCode::KeyW) {
        debug!("Key W pressed, moving cursor up");

        cursor_loc.0.row -= 1;
    } else if keyboard_input.just_released(KeyCode::KeyA) {
        debug!("Key A pressed, moving cursor left");

        cursor_loc.0.col -= 1;
    } else if keyboard_input.just_released(KeyCode::KeyS) {
        debug!("Key S pressed, moving cursor down");

        cursor_loc.0.row += 1;
    } else if keyboard_input.just_released(KeyCode::KeyD) {
        debug!("Key D pressed, moving cursor right");

        cursor_loc.0.col += 1;
    }
}
