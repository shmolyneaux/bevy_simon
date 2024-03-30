use bevy::prelude::*;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use bevy::window::PrimaryWindow;
use bevy::ecs::system::SystemId;
use bevy::sprite::Anchor;

use strum_macros::EnumIter;
use strum::IntoEnumIterator;
use std::collections::HashMap;

use rand::prelude::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;
    use web_sys::window;

    #[wasm_bindgen]
    pub fn save_score(score: u8) {
        let window = window().expect("should have a Window");
        let local_storage = window.local_storage().expect("should have localStorage").expect("localStorage should be available");

        local_storage.set_item("high_score", &score.to_string()).expect("should be able to set localStorage item");
    }

    #[wasm_bindgen]
    #[cfg(target_arch = "wasm32")]
    pub fn load_score() -> u8 {
        let window = window().expect("should have a Window");
        let local_storage = window.local_storage().expect("should have localStorage").expect("localStorage should be available");

        let score_str = local_storage.get_item("high_score").expect("should be able to get localStorage item");
        score_str.unwrap_or_default().parse::<u8>().unwrap_or(0)
    }
}

fn save_score(score: u8) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm::save_score(score)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
          std::fs::write(SAVE_PATH, score.to_string()).unwrap()
    }
}

#[cfg(not(target_arch = "wasm32"))]
const SAVE_PATH: &str = "local.data";

fn load_score() -> u8 {
    #[cfg(target_arch = "wasm32")]
    {
        wasm::load_score()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::read_to_string(SAVE_PATH) {
            Ok(s) => s.parse().unwrap_or(0),
            Err(_) => 0,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug, EnumIter)]
enum Scene {
    Startup,
    ClickToStart,
    MainMenu,
    Game,
    Score,
    Credits,
}

#[derive(Resource)]
struct SceneSetupSystem {
    system_map: HashMap<Scene, SystemId>
}

#[derive(Resource, Default)]
struct GameState {
    pattern: Vec<u8>,
    interactive: bool,
    max_idx: u8,
    idx: u8,
}

impl GameState {
    fn new() -> Self {
        Self::default()
    }

    fn reset(self: &mut Self) {
        *self = Self::default();
    }
}

#[derive(Resource)]
struct HighScore(u8);

#[derive(Resource)]
struct OldHighScore(u8);

#[derive(Resource)]
struct CurrentScene(Scene);

#[derive(Resource)]
struct NextScene(Scene);

#[derive(Resource, Debug)]
struct ShmMousePosition {
    pos: Option<Vec2>,
}

#[derive(Resource)]
struct PatternAnimationTimer(Timer);

#[derive(Resource)]
struct PatternSounds(Handle<AudioSource>, Handle<AudioSource>, Handle<AudioSource>, Handle<AudioSource>);

#[derive(Component)]
struct PatternIdx(u8);

#[derive(Component)]
struct SceneChangeButton {
    width: f32,
    height: f32,
    scene: Scene,
}

#[derive(Component)]
struct SceneObject(());

enum HoverShape {
    Rectangle(Vec2),
    Triangle(Vec2, Vec2, Vec2),
}

#[derive(Component)]
struct MouseHoverDisable;

#[derive(Component)]
struct MouseHoverTracker {
    is_hovered: bool,
    is_just_hovered: bool,
    is_just_unhovered: bool,
    shape: HoverShape
}

#[derive(Component)]
struct MouseOverMaterial(Handle<ColorMaterial>);

#[derive(Component)]
struct MouseOutMaterial(Handle<ColorMaterial>);

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct MemorizeLabel;

impl MouseHoverTracker {
    fn from_rect(w: f32, h: f32) -> Self {
        Self {
            shape: HoverShape::Rectangle(Vec2::new(w/2., h/2.)),
            is_hovered: false,
            is_just_hovered: false,
            is_just_unhovered: false,
        }
    }

    fn from_triangle(a: Vec2, b: Vec2, c: Vec2) -> Self {
        Self {
            shape: HoverShape::Triangle(a, b, c),
            is_hovered: false,
            is_just_hovered: false,
            is_just_unhovered: false,
        }
    }

    fn set_hovered(self: &mut Self, is_hovered: bool) {
        if self.is_hovered != is_hovered {
            self.is_hovered = is_hovered;

            self.is_just_hovered = false;
            self.is_just_unhovered = false;
            if is_hovered {
                self.is_just_hovered = true;
            } else {
                self.is_just_unhovered = true;
            }
        } else {
            self.is_just_hovered = false;
            self.is_just_unhovered = false;
        }
    }
}

fn check_collision_point_tri(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    // Get the barycentric coordinates and check that they're within the triangle
    // TODO: Maybe this could be faster if the coordinates weren't normalized
    // to the size of the triangle?
    let inv_triangle_area = ((b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y)).recip();

    let bary_a_area = (b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y);
    let bary_b_area = (c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y);

    let bary_a = bary_a_area * inv_triangle_area;
    let bary_b = bary_b_area * inv_triangle_area;
    let bary_c = 1. - bary_a - bary_b;

    (bary_a > 0.) && (bary_b > 0.) && (bary_c > 0.)
}

fn setup(
    world: &mut World,
) {
    world.spawn((Camera2dBundle::default(), MainCamera));

    let mut system_map = HashMap::new();
    for scene in Scene::iter() {
        if let Some(system_id) = match scene {
            Scene::Startup => None,
            Scene::ClickToStart => Some(world.register_system(setup_click_to_start_scene)),
            Scene::MainMenu => Some(world.register_system(setup_main_menu)),
            Scene::Credits => Some(world.register_system(setup_credits)),
            Scene::Game => Some(world.register_system(setup_game)),
            Scene::Score => Some(world.register_system(setup_score)),
        } {
            system_map.insert(scene, system_id);
        }
    }

    let setup_systems = SceneSetupSystem { system_map };
    world.insert_resource(setup_systems);
}

fn load_assets(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    commands.insert_resource(
        PatternSounds(
            asset_server.load("sounds/drop_003_p0.ogg"),
            asset_server.load("sounds/drop_003_p1.ogg"),
            asset_server.load("sounds/drop_003_p2.ogg"),
            asset_server.load("sounds/drop_003_p3.ogg"),
        )
    );
}

fn setup_click_to_start_scene(
    window: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let window = window.single();

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::BLACK,
    };
    let text_justification = JustifyText::Center;

    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        MouseHoverTracker::from_rect(99999., 99999.),
        SceneChangeButton {
            width: 99999.,
            height: 99999.,
            scene: Scene::MainMenu,
        },
        SceneObject(()),
    ));

    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Click anywhere to begin", text_style.clone())
                .with_justify(text_justification),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        SceneObject(()),
    ));
}

fn setup_main_menu(
    window: Query<&Window, With<PrimaryWindow>>,
    high_score: Res<HighScore>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = window.single();

    add_scene_change_button(
        &asset_server,
        &mut commands,
        &mut materials,
        &mut meshes,
        "Start Game",
        Color::rgb_u8(0, 228, 48),
        Color::rgb_u8(0, 117, 44),
        275.0,
        60.0,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Scene::Game,
    );

    // Credits button
    add_scene_change_button(
        &asset_server,
        &mut commands,
        &mut materials,
        &mut meshes,
        "Credits",
        Color::rgb_u8(0, 121, 241),
        Color::rgb_u8(0, 82, 172),
        180.0,
        60.0,
        Transform::from_xyz(0.0, -80.0, 0.0),
        Scene::Credits,
    );

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 80.0,
        color: Color::BLACK,
    };

    let bl = Vec2::new(-window.resolution.width()/2., -window.resolution.height()/2.);

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(&format!("High Score: {}", high_score.0), text_style.clone()),
            text_anchor: Anchor::BottomLeft,
            transform: Transform::from_xyz(bl.x+10., bl.y, 0.0),
            ..default()
        },
        SceneObject(()),
    ));
}

fn setup_credits(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 80.0,
        color: Color::BLACK,
    };
    let text_justification = JustifyText::Center;

    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Game by Stephen Molyneaux 2024", text_style.clone())
                .with_justify(text_justification),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        MouseHoverTracker::from_rect(99999., 99999.),
        SceneChangeButton {
            width: 99999.,
            height: 99999.,
            scene: Scene::MainMenu,
        },
        SceneObject(()),
    ));

    let text_style = TextStyle {
        font: font.clone(),
        font_size: 40.0,
        color: Color::BLACK,
    };
    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Created with Bevy", text_style.clone())
                .with_justify(text_justification),
            transform: Transform::from_xyz(0.0, -80.0, 0.0),
            ..default()
        },
        SceneObject(()),
    ));

    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::BLACK,
    };
    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Click to Return", text_style.clone())
                .with_justify(text_justification),
            transform: Transform::from_xyz(0.0, -220.0, 0.0),
            ..default()
        },
        SceneObject(()),
    ));
}

fn setup_game(
    asset_server: Res<AssetServer>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut timer: ResMut<PatternAnimationTimer>,
    mut commands: Commands,
    mut state: ResMut<GameState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = window.single();

    timer.0.reset();
    state.reset();
    for idx in 0..255 {
        state.pattern.push(rand::random::<u8>() % 4);
    }
    state.max_idx = 0;

    // Create 4 touch areas
    let red = Color::hsl(0.0, 0.95, 0.9);
    let hover_red = Color::hsl(0.0, 0.95, 0.8);
    let button_red = Color::hsl(0.0, 0.95, 0.4);
    let green = Color::hsl(115.0, 0.95, 0.9);
    let hover_green = Color::hsl(115.0, 0.95, 0.8);
    let blue = Color::hsl(235.0, 0.95, 0.9);
    let hover_blue = Color::hsl(235.0, 0.95, 0.8);
    let yellow = Color::hsl(60.0, 0.95, 0.9);
    let hover_yellow = Color::hsl(60.0, 0.95, 0.8);

    let center = Vec2::new(0., 0.);
    let tl = Vec2::new(-window.resolution.width()/2., window.resolution.height()/2.);
    let tr = Vec2::new(window.resolution.width()/2., window.resolution.height()/2.);
    let bl = Vec2::new(-window.resolution.width()/2., -window.resolution.height()/2.);
    let br = Vec2::new(window.resolution.width()/2., -window.resolution.height()/2.);

    // Red button
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: Mesh2dHandle(
                meshes.add(
                    Triangle2d::new(center, tl, tr)
                )
            ),
            material: materials.add(red),
            transform: Transform::from_xyz(0., 0., 0.),
            ..default()
        },
        MouseHoverDisable,
        MouseHoverTracker::from_triangle(center, tl, tr),
        MouseOverMaterial(materials.add(hover_red)),
        MouseOutMaterial(materials.add(red)),
        PatternIdx(0),
        SceneObject(()),
    ));

    // Green button
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: Mesh2dHandle(
                meshes.add(
                    Triangle2d::new(center, tr, br)
                )
            ),
            material: materials.add(green),
            transform: Transform::from_xyz(0., 0., 0.),
            ..default()
        },
        MouseHoverDisable,
        MouseHoverTracker::from_triangle(center, tr, br),
        MouseOverMaterial(materials.add(hover_green)),
        MouseOutMaterial(materials.add(green)),
        PatternIdx(1),
        SceneObject(()),
    ));

    // Blue button
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: Mesh2dHandle(
                meshes.add(
                    Triangle2d::new(center, bl, br)
                )
            ),
            material: materials.add(blue),
            transform: Transform::from_xyz(0., 0., 0.),
            ..default()
        },
        MouseHoverDisable,
        MouseHoverTracker::from_triangle(center, bl, br),
        MouseOverMaterial(materials.add(hover_blue)),
        MouseOutMaterial(materials.add(blue)),
        PatternIdx(2),
        SceneObject(()),
    ));

    // Yellow button
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: Mesh2dHandle(
                meshes.add(
                    Triangle2d::new(center, tl, bl)
                )
            ),
            material: materials.add(yellow),
            transform: Transform::from_xyz(0., 0., 0.),
            ..default()
        },
        MouseHoverDisable,
        MouseHoverTracker::from_triangle(center, tl, bl),
        MouseOverMaterial(materials.add(hover_yellow)),
        MouseOutMaterial(materials.add(yellow)),
        PatternIdx(3),
        SceneObject(()),
    ));

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 80.0,
        color: Color::BLACK,
    };
    let text_justification = JustifyText::Center;

    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Memorize", text_style.clone())
                .with_justify(text_justification),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        MemorizeLabel,
        SceneObject(()),
    ));
}

fn pattern_playback_system(
    mut commands: Commands,
    sounds: Res<PatternSounds>,
    time: Res<Time>,
    mut query: Query<(Entity, &PatternIdx, &MouseOverMaterial, &MouseOutMaterial, &mut Handle<ColorMaterial>)>,
    mut label: Query<Entity, With<MemorizeLabel>>,
    mut timer: ResMut<PatternAnimationTimer>,
    mut state: ResMut<GameState>,
) {
    if !state.interactive {
        if timer.0.tick(time.delta()).just_finished() {
            println!("PB system timer just finished");
            if state.idx > state.max_idx {
                state.interactive = true;
                state.idx = 0;
                for (entity_id, idx, over, out, mut mat) in &mut query {
                    *mat = out.0.clone();
                    commands.entity(entity_id).remove::<MouseHoverDisable>();
                }
                for entity_id in &label {
                    commands.entity(entity_id).insert(Visibility::Hidden);
                }
            } else {
                if !state.pattern.is_empty() {
                    println!(
                        "Playing sound {} for idx {}",
                        state.pattern[state.idx as usize],
                        state.idx,
                    );
                    commands.spawn(AudioBundle {
                        source: match state.pattern[state.idx as usize] {
                            0 => sounds.0.clone(),
                            1 => sounds.1.clone(),
                            2 => sounds.2.clone(),
                            _ => sounds.3.clone(),
                        },
                        settings: PlaybackSettings::DESPAWN,
                    });
                }
                for (entity_id, idx, over, out, mut mat) in &mut query {
                    if state.pattern[state.idx as usize] == idx.0 {
                        *mat = over.0.clone();
                    } else {
                        *mat = out.0.clone();
                    }
                }
                state.idx += 1;
            }
        }
    }
}

fn user_game_system(
    mut commands: Commands,
    sounds: Res<PatternSounds>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<ShmMousePosition>,
    mut next_scene: ResMut<NextScene>,
    mut query: Query<(Entity, &MouseHoverTracker, &PatternIdx)>,
    mut timer: ResMut<PatternAnimationTimer>,
    mut state: ResMut<GameState>,
    mut label: Query<Entity, With<MemorizeLabel>>,
) {
    if state.interactive && mouse.just_released(MouseButton::Left) {
        let mut button_idx = None;
        for (_entity, tracker, idx) in &query {
            if tracker.is_hovered {
                button_idx = Some(idx.0);
                break;
            }
        }

        if button_idx.is_none() {
            return;
        }

        let button_idx = button_idx.unwrap();

        if button_idx == state.pattern[state.idx as usize] {
            // We pressed the right button
            commands.spawn(AudioBundle {
                source: match button_idx {
                    0 => sounds.0.clone(),
                    1 => sounds.1.clone(),
                    2 => sounds.2.clone(),
                    _ => sounds.3.clone(),
                },
                settings: PlaybackSettings::DESPAWN,
            });
            if state.idx == state.max_idx {
                state.idx = 0;
                state.max_idx += 1;
                state.interactive = false;
                timer.0.reset();
                for entity_id in &label {
                    commands.entity(entity_id).insert(Visibility::Visible);
                }
                for (entity_id, _tracker, _idx) in &query {
                    commands.entity(entity_id).insert(MouseHoverDisable);
                }
            } else {
                state.idx += 1;
            }
        } else {
            // We pressed the wrong button
            let settings = PlaybackSettings::DESPAWN;
            commands.spawn(AudioBundle {settings, source: sounds.0.clone()});
            commands.spawn(AudioBundle {settings, source: sounds.1.clone()});
            commands.spawn(AudioBundle {settings, source: sounds.2.clone()});
            commands.spawn(AudioBundle {settings, source: sounds.3.clone()});

            next_scene.0 = Scene::Score;
        }
    }
}

fn setup_score(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    state: Res<GameState>,
    mut old_high_score: ResMut<OldHighScore>,
    mut high_score: ResMut<HighScore>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 80.0,
        color: Color::BLACK,
    };
    let text_justification = JustifyText::Center;

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(format!("Score: {}", state.max_idx), text_style.clone())
                .with_justify(text_justification),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        SceneObject(()),
    ));

    if state.max_idx > high_score.0 {
        old_high_score.0 = high_score.0;
        high_score.0 = state.max_idx;
        save_score(high_score.0);
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(&format!("NEW HIGH SCORE!"), text_style.clone())
                    .with_justify(text_justification),
                transform: Transform::from_xyz(0.0, 80.0, 0.0),
                ..default()
            },
            SceneObject(()),
        ));
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(&format!("Old High Score: {}", old_high_score.0), text_style.clone())
                    .with_justify(text_justification),
                transform: Transform::from_xyz(0.0, -80.0, 0.0),
                ..default()
            },
            SceneObject(()),
        ));
    } else {
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(&format!("High Score: {}", high_score.0), text_style.clone())
                    .with_justify(text_justification),
                transform: Transform::from_xyz(0.0, -80.0, 0.0),
                ..default()
            },
            SceneObject(()),
        ));
    }

    add_scene_change_button(
        &asset_server,
        &mut commands,
        &mut materials,
        &mut meshes,
        "Click to return",
        Color::hsl(235.0, 0.95, 0.7),
        Color::hsl(235.0, 0.95, 0.3),
        500.0,
        60.0,
        Transform::from_xyz(0.0, -240.0, 0.0),
        Scene::MainMenu,
    );
}

fn add_scene_change_button(
    asset_server: &Res<AssetServer>,
    commands: &mut Commands,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    text: &str,
    color: Color,
    hover_color: Color,
    // TODO: need to use UI system to measure text size...
    width: f32,
    height: f32,
    mut transform: Transform,
    scene: Scene,
) {
    // TODO: somehow it seems like this should be one entity...
    // But the text needs a different transform (Z) than the rectangle
    // Somehow you can create things with children, but that's a problem for
    // another time

    // Button rectangle
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: Mesh2dHandle(meshes.add(Rectangle::new(width, height))),
            material: materials.add(color),
            transform: transform,
            ..default()
        },
        MouseHoverTracker::from_rect(width, height),
        MouseOverMaterial(materials.add(hover_color)),
        MouseOutMaterial(materials.add(color)),
        SceneObject(()),
    ));

    // Button text/action
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::BLACK,
    };
    let text_justification = JustifyText::Center;
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::BLACK,
    };
    let text_justification = JustifyText::Center;

    transform.translation += Vec3::new(0.0, 0.0, 1.0);

    commands.spawn((
        Text2dBundle {
            text: Text::from_section(text, text_style.clone())
                .with_justify(text_justification),
            transform: transform,
            ..default()
        },
        MouseHoverTracker::from_rect(width, height),
        SceneChangeButton {
            width,
            height,
            scene,
        },
        SceneObject(()),
    ));
}

fn handle_scene_change(
    next_scene: Res<NextScene>,
    scene_setup_system: Res<SceneSetupSystem>,
    scene_objects: Query<Entity, With<SceneObject>>,
    mut commands: Commands,
    mut current_scene: ResMut<CurrentScene>,
) {
    // Check if we're updating the scene
    if next_scene.0 != current_scene.0 {
        let scene = next_scene.0;
        current_scene.0 = scene;

        println!("Switching to {scene:?}");

        // Remove any scene-specific entities
        println!("Removing scene objects");
        for obj in &scene_objects {
            commands.entity(obj).despawn();
        }

        // Run the setup system for the new scene
        if let Some(system) = scene_setup_system.system_map.get(&scene) {
            commands.run_system(*system);
        } else {
            println!("NOTE: Transitioning to scene {scene:?} which does not have a setup system");
        }
    }
}

fn update_mouse_position(
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut mouse: ResMut<ShmMousePosition>,
) {
    let (camera, camera_transform) = camera.single();
    let window = window.single();

    if let Some(world_position) = window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        mouse.pos = Some(world_position);
    } else {
        mouse.pos = None;
    }
}

fn update_mouse_hover_disable(
    mut tracked_objects: Query<&mut MouseHoverTracker, With<MouseHoverDisable>>,
) {
    for mut tracker in &mut tracked_objects {
        tracker.set_hovered(false);
    }
}

fn update_mouse_hover_state(
    mouse: ResMut<ShmMousePosition>,
    mut tracked_objects: Query<(&mut MouseHoverTracker, &Transform), Without<MouseHoverDisable>>,
) {
    if let Some(mouse_pos) = mouse.pos {
        for (mut tracker, transform) in &mut tracked_objects {
            let local_mouse_pos = transform.compute_matrix().inverse().transform_point3(mouse_pos.extend(0.0)).xy();
            let hovered = match tracker.shape {
                HoverShape::Rectangle(r) => {
                    -r.x <= local_mouse_pos.x && local_mouse_pos.x <= r.x
                        && -r.y <= local_mouse_pos.y && local_mouse_pos.y <= r.y
                }
                HoverShape::Triangle(a, b, c) => {
                    check_collision_point_tri(local_mouse_pos, a, b, c)
                }
            };
            tracker.set_hovered(hovered);
        }
    } else {
        for (mut tracker, _transform) in &mut tracked_objects {
            tracker.set_hovered(false);
        }
    }
}

fn update_mouse_hover_material(
    mut query: Query<(&MouseHoverTracker, &MouseOverMaterial, &mut Handle<ColorMaterial>)>,
) {
    for (tracker, material_info, mut material) in &mut query {
        if tracker.is_just_hovered {
            *material = material_info.0.clone();
        }
    }
}

fn update_mouse_unhover_material(
    mut query: Query<(&MouseHoverTracker, &MouseOutMaterial, &mut Handle<ColorMaterial>)>,
) {
    for (tracker, material_info, mut material) in &mut query {
        if tracker.is_just_unhovered {
            *material = material_info.0.clone();
        }
    }
}

fn scene_change_button(
    query: Query<(&SceneChangeButton, &MouseHoverTracker)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_pos: Res<ShmMousePosition>,
    mut next_scene: ResMut<NextScene>,
) {
    // If we just click the mouse button in frame, find if any scene change
    // buttons were hovered.
    if mouse.just_released(MouseButton::Left) {
        if let Some(mouse_pos) = mouse_pos.pos {
            for (button, tracker) in &query {
                if tracker.is_hovered {
                    println!("Requesting switch to {:?}", button.scene);
                    next_scene.0 = button.scene;
                    break;
                }
            }
        }
    }
}

pub fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}

pub struct ShmPlugin;
impl Plugin for ShmPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentScene(Scene::Startup))
            .insert_resource(ClearColor(Color::rgb_u8(245, 245, 245)))
            .insert_resource(NextScene(Scene::ClickToStart))
            .insert_resource(GameState::new())
            .insert_resource(ShmMousePosition { pos: None })
            .insert_resource(PatternAnimationTimer(Timer::from_seconds(1.0, TimerMode::Repeating)))
            .insert_resource(HighScore(load_score()))
            .insert_resource(OldHighScore(0))
            .add_systems(Startup, (setup, load_assets).chain())
            .add_systems(
                Update,
                (
                    update_mouse_position,
                    update_mouse_hover_state,
                    update_mouse_hover_disable,
                    update_mouse_hover_material,
                    update_mouse_unhover_material,
                    pattern_playback_system,
                    user_game_system,
                    scene_change_button,
                    handle_scene_change,
                    close_on_esc,
                )
                    .chain(),
            );
    }
}

fn main() {
    App::new().add_plugins((DefaultPlugins, ShmPlugin)).run();
}
