#![feature(let_chains)]
use anyhow::Result;
use bevy::{app::AppExit, prelude::*, window::PresentMode};
use crossbeam::channel::Sender;
use player::Player;
use rodio::OutputStream;
use state::{AppState, MainScreenMode};
use std::sync::{Arc, Mutex};
use synth_lib::audio::{AudioOutputSync, TrackerSynth};
use tracker_lib::{PlayerCmd, TrackerState};

mod main_menu;
mod player;
mod setup;
pub mod state;

#[derive(Resource)]
pub struct Synth(Arc<Mutex<TrackerSynth>>);

#[derive(Resource)]
pub struct PlayerSynth(Player);

#[derive(Resource)]
pub struct PlayerIpc(Sender<PlayerCmd>);

fn main() -> Result<()> {
    let synth = Arc::new(Mutex::new(TrackerSynth::default()));
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let audio = AudioOutputSync {
        synth: synth.clone(),
        _stream,
    };

    info!("starting audio stream");
    stream_handle.play_raw(audio).unwrap();
    info!("initializing tracker state");
    let state = TrackerState::default();
    info!("initializing player");
    let (player, player_ipc) = Player::new(synth.clone());

    App::new()
        .insert_resource(state)
        .insert_resource(Synth(synth))
        .insert_resource(PlayerSynth(player))
        .insert_resource(PlayerIpc(player_ipc))
        .add_plugins(setup::SetupPlugin)
        .add_plugins(main_menu::MainMenuPlugin)
        // .add_plugins(player::PlayerPlugin)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // title: "I am a window!".into(),
                        // name: Some("bevy.app".into()),
                        name: Some("rusty-tracker".into()),
                        resolution: (500., 281.).into(),
                        present_mode: PresentMode::AutoVsync,
                        // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                        prevent_default_event_handling: false,
                        enabled_buttons: bevy::window::EnabledButtons {
                            maximize: false,
                            ..Default::default()
                        },
                        // This will spawn an invisible window
                        // The window will be made visible in the make_visible() system after 3 frames.
                        // This is useful when you want to avoid the white window that shows up before the GPU is ready to render the app.
                        visible: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_systems(OnEnter(AppState::ExitTracker), exit_game)
        .init_state::<AppState>()
        .add_sub_state::<MainScreenMode>()
        .run();

    Ok(())
}

fn exit_game(mut exit: EventWriter<AppExit>) {
    exit.send(AppExit::Success);
}
