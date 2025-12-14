use bevy::prelude::*;
use bevy::window::PresentMode;
use std::sync::{Arc, atomic::AtomicU64};
use parking_lot::Mutex;

// Import library modules
use oxidizer_core::audio::mixer::AudioMixer;
use oxidizer_core::audio::backend::AudioStream;
use oxidizer_core::time::conductor::Conductor;
use oxidizer_core::audio::mixer::MixerHandle;
use oxidizer_core::input::InputQueue;
use oxidizer_core::chart::Chart;
use oxidizer_core::gameplay::judge::JudgeMachine;

// Resources
#[derive(Resource)]
struct GameConductor(Conductor);

#[derive(Resource)]
struct GameMixer(MixerHandle);

#[derive(Resource)]
struct GameInputQueue(InputQueue);

#[derive(Resource)]
struct GameChart(Chart);

#[derive(Resource)]
struct GameJudge(JudgeMachine);

#[derive(Resource, Default)]
struct Score {
    perfect: usize,
    great: usize,
    good: usize,
    bad: usize,
    poor: usize,
    miss: usize,
    combo: usize,
}

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
struct VisualNote {
    note_index: usize, // Index in Chart.notes
    target_time: f64,
    lane: usize,
}

fn main() {
    let processed_samples = Arc::new(AtomicU64::new(0));
    let (mixer, mixer_handle) = AudioMixer::new(44100);
    let mixer = Arc::new(Mutex::new(mixer));

    let _stream = AudioStream::new(mixer, processed_samples.clone())
        .expect("Failed to initialize AudioStream");

    let conductor = Conductor::new(processed_samples, 44100);
    let input_queue = InputQueue::new();
    let chart = Chart::dummy();
    let judge_machine = JudgeMachine::new();

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
        .insert_resource(GameInputQueue(input_queue))
        .insert_resource(GameChart(chart))
        .insert_resource(GameJudge(judge_machine))
        .insert_resource(Score::default())
        .insert_resource(ScrollConfig::default())
        .add_systems(Startup, (setup_time_display, setup_camera, spawn_notes))
        .add_systems(PreUpdate, update_conductor_system)
        .add_systems(Update, (update_time_display, move_notes, judgment_system, miss_detection_system))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_time_display(mut commands: Commands) {
    commands.spawn((
        TextBundle::from_section(
            "Time: 0.00 | Score: 0/0/0/0/0",
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

fn spawn_notes(mut commands: Commands, chart: Res<GameChart>) {
    // Spawn notes from chart
    for (i, note) in chart.0.notes.iter().enumerate() {
        // Lane 0 = Scratch, 1-7 Keys
        // Visual separation: Scratch left (-?), keys center/right.
        // Simple visualization:
        // x = (lane - 4) * 50.0 (roughly centered)
        let x_pos = (note.lane as f32 - 4.0) * 50.0;

        let color = if note.lane == 0 {
            Color::srgb(1.0, 0.0, 0.0) // Scratch (Red)
        } else if note.lane % 2 == 1 {
            Color::srgb(1.0, 1.0, 1.0) // White keys
        } else {
            Color::srgb(0.0, 0.0, 1.0) // Black keys (Blue)
            // IIDX: 1,3,5,7 White. 2,4,6 Black.
            // Lane 1=1, Lane 2=2...
        };

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(40.0, 10.0)),
                    ..default()
                },
                transform: Transform::from_xyz(x_pos, -1000.0, 0.0), // Start off screen
                ..default()
            },
            VisualNote {
                note_index: i,
                target_time: note.time,
                lane: note.lane,
            },
        ));
    }
}

fn update_conductor_system(time: Res<Time<Real>>, mut conductor: ResMut<GameConductor>) {
    conductor.0.update(time.elapsed_seconds_f64());
}

fn update_time_display(
    time: Res<Time<Real>>,
    conductor: Res<GameConductor>,
    score: Res<Score>,
    mut query: Query<&mut Text, With<TimeDisplay>>,
) {
    let current_time = conductor.0.get_time(time.elapsed_seconds_f64());
    for mut text in &mut query {
        text.sections[0].value = format!(
            "Time: {:.3} | PG:{}/G:{}/Gd:{}/B:{}/P:{}",
            current_time, score.perfect, score.great, score.good, score.bad, score.poor
        );
    }
}

fn move_notes(
    time: Res<Time<Real>>,
    conductor: Res<GameConductor>,
    config: Res<ScrollConfig>,
    mut query: Query<(&VisualNote, &mut Transform)>,
) {
    let current_time = conductor.0.get_time(time.elapsed_seconds_f64());

    let visible_height = config.lane_height - config.sud_plus - config.lift;
    let gn_seconds = config.green_number / 1000.0;
    let pixels_per_sec = if gn_seconds > 0.0 {
        visible_height / gn_seconds
    } else {
        0.0
    };

    for (note, mut transform) in &mut query {
        let time_diff = note.target_time - current_time;
        // y_position = config.lift + (time_diff * pixels_per_sec)
        let y_position = config.lift + (time_diff as f32 * pixels_per_sec);

        transform.translation.y = y_position;
    }
}

fn judgment_system(
    input_queue: Res<GameInputQueue>,
    chart: Res<GameChart>,
    mut judge: ResMut<GameJudge>,
    mut score: ResMut<Score>,
    mut commands: Commands,
    query: Query<(Entity, &VisualNote)>,
) {
    // Process all pending inputs
    while let Some(event) = input_queue.0.pop() {
        if let Some(result) = judge.0.process_input(event, &chart.0) {
            println!("Judgment: {:?}", result);

            // Update score
            match result.judgment {
                oxidizer_core::chart::Judgment::Perfect => { score.perfect += 1; score.combo += 1; },
                oxidizer_core::chart::Judgment::Great => { score.great += 1; score.combo += 1; },
                oxidizer_core::chart::Judgment::Good => { score.good += 1; score.combo += 1; },
                oxidizer_core::chart::Judgment::Bad => { score.bad += 1; score.combo = 0; },
                oxidizer_core::chart::Judgment::Poor | oxidizer_core::chart::Judgment::Miss => { score.poor += 1; score.combo = 0; },
            }

            // Visual feedback: Despawn the note
            // Find entity with matching note_index
            for (entity, note) in &query {
                if note.note_index == result.note_index {
                    // Despawn note
                    commands.entity(entity).despawn();
                    break;
                }
            }
        }
    }
}

fn miss_detection_system(
    time: Res<Time<Real>>,
    conductor: Res<GameConductor>,
    chart: Res<GameChart>,
    mut judge: ResMut<GameJudge>,
    mut score: ResMut<Score>,
    mut commands: Commands,
    query: Query<(Entity, &VisualNote)>,
) {
    let current_time = conductor.0.get_time(time.elapsed_seconds_f64());
    let misses = judge.0.check_misses(current_time, &chart.0);

    for miss in misses {
        println!("Miss: {:?}", miss);
        score.poor += 1;
        score.combo = 0;

        // Find entity and despawn (or mark as missed)
        for (entity, note) in &query {
            if note.note_index == miss.note_index {
                // Despawn note for now (or change color to red/gray)
                // Let's despawn to clean up
                commands.entity(entity).despawn();
                break;
            }
        }
    }
}
