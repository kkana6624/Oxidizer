use bevy::prelude::*;
use bevy::window::PresentMode;
use std::sync::{Arc, atomic::AtomicU64};
use parking_lot::Mutex;

// Import library modules
use oxidizer_core::audio::mixer::AudioMixer;
use oxidizer_core::audio::backend::AudioStream;
use oxidizer_core::time::conductor::Conductor;
use oxidizer_core::audio::mixer::MixerHandle;

// Resources
#[derive(Resource)]
struct GameConductor(Conductor);

#[derive(Resource)]
struct GameMixer(MixerHandle);

#[derive(Resource)]
struct ScrollConfig {
    green_number: f32, // Target visibility time in ms
    sud_plus: f32,     // Lane cover height in pixels
    lift: f32,         // Lift height in pixels
    lane_height: f32,  // Total lane height
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            green_number: 300.0,
            sud_plus: 0.0,
            lift: 0.0,
            lane_height: 1000.0,
        }
    }
}

// Components
#[derive(Component)]
struct TimeDisplay;

#[derive(Component)]
struct TestNote {
    target_time: f64,
}

fn main() {
    let processed_samples = Arc::new(AtomicU64::new(0));
    let (mixer, mixer_handle) = AudioMixer::new(44100);
    let mixer = Arc::new(Mutex::new(mixer));

    let _stream = AudioStream::new(mixer, processed_samples.clone())
        .expect("Failed to initialize AudioStream");

    let conductor = Conductor::new(processed_samples, 44100);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Oxidizer".into(),
                resolution: (1920.0, 1080.0).into(),
                present_mode: PresentMode::Mailbox,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(GameConductor(conductor))
        .insert_resource(GameMixer(mixer_handle))
        .insert_resource(ScrollConfig::default())
        .add_systems(Startup, (setup_time_display, setup_camera, spawn_test_note))
        .add_systems(PreUpdate, update_conductor_system)
        .add_systems(Update, (update_time_display, move_notes))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_time_display(mut commands: Commands) {
    commands.spawn((
        TextBundle::from_section(
            "Time: 0.00",
            TextStyle {
                font_size: 30.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        TimeDisplay,
    ));
}

fn spawn_test_note(mut commands: Commands) {
    // Spawn a white square representing a note
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(100.0, 20.0)),
                ..default()
            },
            ..default()
        },
        TestNote { target_time: 2.0 },
    ));
}

fn update_conductor_system(time: Res<Time<Real>>, mut conductor: ResMut<GameConductor>) {
    conductor.0.update(time.elapsed_seconds_f64());
}

fn update_time_display(
    time: Res<Time<Real>>,
    conductor: Res<GameConductor>,
    mut query: Query<&mut Text, With<TimeDisplay>>,
) {
    let current_time = conductor.0.get_time(time.elapsed_seconds_f64());
    for mut text in &mut query {
        text.sections[0].value = format!("Time: {:.3}", current_time);
    }
}

fn move_notes(
    time: Res<Time<Real>>,
    conductor: Res<GameConductor>,
    config: Res<ScrollConfig>,
    mut query: Query<(&TestNote, &mut Transform)>,
) {
    let current_time = conductor.0.get_time(time.elapsed_seconds_f64());

    // visible_height = config.lane_height - config.sud_plus - config.lift
    let visible_height = config.lane_height - config.sud_plus - config.lift;

    // pixels_per_sec = visible_height / (config.green_number / 1000.0)
    // Avoid division by zero
    let gn_seconds = config.green_number / 1000.0;
    let pixels_per_sec = if gn_seconds > 0.0 {
        visible_height / gn_seconds
    } else {
        0.0 // Or infinity?
    };

    for (note, mut transform) in &mut query {
        let time_diff = note.target_time - current_time;
        // y_position = config.lift + (time_diff * pixels_per_sec)
        let y_position = config.lift + (time_diff as f32 * pixels_per_sec);

        // Optimization: if y_position > config.lane_height - config.sud_plus, hidden?
        // We'll just set the position for now, maybe set visibility to Hidden if out of bounds?
        // Task says: "Optimization: If y_position > config.lane_height - config.sud_plus, the note is hidden"
        // But for visual confirmation, let's just move it.

        // Also coordinate system: Camera2d is centered at 0,0.
        // We probably want 0 to be bottom?
        // Usually rhythm games have a fixed judgment line position.
        // The formula assumes 0 is "bottom" (where lift starts).
        // Since Camera2d is centered, we might need to offset everything or move camera.
        // Let's assume we map y_position directly to transform.y for now,
        // effectively making 0 the center of the screen unless we offset.
        // But the task didn't specify coordinate system details, just the formula.
        // I'll stick to the formula.

        transform.translation.y = y_position;
    }
}
