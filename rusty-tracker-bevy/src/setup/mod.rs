use crate::state::{AppState, StyleState};
use bevy::{core::FrameCount, prelude::*, window::PrimaryWindow};
// use tracker_lib::TrackerState;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_systems(Startup, setup_camera)
            .insert_resource(StyleState::default())
            .add_systems(OnExit(AppState::Setup), setup_camera)
            .add_systems(Update, make_visible.run_if(in_state(AppState::Setup)));
    }
}

#[derive(Component)]
struct GameView;

fn setup_camera(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<Entity, With<GameView>>,
    style: Res<StyleState>,
) {
    use bevy::render::camera::ScalingMode;

    camera_query
        .iter()
        .for_each(|cam| commands.entity(cam).despawn());

    let window = window_query.get_single().unwrap();

    let mut camera = Camera2dBundle {
        // clear the whole viewport with the given color
        camera: Camera {
            clear_color: ClearColorConfig::Custom(style.palette.colors.base.into()),
            ..Default::default()
        },
        ..Default::default()
    };

    // TODO: figure out if the below is actually needed
    camera.transform = Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 0.0);
    camera.projection.scaling_mode = ScalingMode::FixedHorizontal(16.0 * 32.0);

    commands.spawn((camera, GameView));
}

// fn init_tracker(mut commands: Commands) {
//     commands.spawn(TrackerState::default());
// }

fn make_visible(
    mut window: Query<&mut Window>,
    frames: Res<FrameCount>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if frames.0 == 5 {
        debug!("making window visible");
        window.single_mut().visible = true;
        next_state.set(AppState::Main);
        info!("transitioning into main state");
    }
}
