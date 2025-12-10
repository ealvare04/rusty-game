// A simple UI/HUD showing player HP and controls
use bevy::prelude::*;

use crate::characters::combat::{CombatState, CombatStats};
use crate::characters::npc::Enemy;

// Simple, always-on HUD showing player HP and controls
#[derive(Component)]
pub struct HudRoot;
#[derive(Component)]
pub struct HudHealthFill;
#[derive(Component)]
pub struct HudEnemyHealthFill;
#[derive(Component)]
pub struct HudHealthBg;
#[derive(Component)]
pub struct HudEnemyHealthBg;
#[derive(Component)]
pub struct HudControlsText;

const HUD_Z: f32 = 55.0; // below combat UI (60+) and overlays (70+)

pub fn spawn_hud_once(
    mut commands: Commands,
    existing: Query<Entity, With<HudRoot>>,
    cam_q: Query<&Transform, With<Camera2d>>,
) {
    if existing.iter().next().is_some() { return; }
    let cam_pos = cam_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);

    // Root marker (no parenting to keep it simple with camera-follow)
    // Place the health bars at the top of the window
    let base = Vec3::new(cam_pos.x, cam_pos.y + 220.0, HUD_Z);
    commands.spawn((Transform::from_translation(base), HudRoot));

    // Health bar background frames (grey) — Player left, Enemy right
    commands.spawn((
        Sprite { color: Color::srgb(0.2, 0.2, 0.2), custom_size: Some(Vec2::new(220.0, 16.0)), ..default() },
        Transform::from_translation(base + Vec3::new(-130.0, 0.0, 0.5)),
        HudHealthBg,
    ));
    commands.spawn((
        Sprite { color: Color::srgb(0.2, 0.2, 0.2), custom_size: Some(Vec2::new(220.0, 16.0)), ..default() },
        Transform::from_translation(base + Vec3::new(130.0, 0.0, 0.5)),
        HudEnemyHealthBg,
    ));

    // Player health bar fill (green), width adjusted in update
    commands.spawn((
        Sprite { color: Color::srgb(0.1, 0.8, 0.1), custom_size: Some(Vec2::new(214.0, 12.0)), ..default() },
        Transform::from_translation(base + Vec3::new(-130.0, 0.0, 0.6)),
        HudHealthFill,
    ));

    // Enemy health bar fill (red), width adjusted in update (hidden if no combat)
    commands.spawn((
        Sprite { color: Color::srgb(0.8, 0.1, 0.1), custom_size: Some(Vec2::new(0.0, 12.0)), ..default() },
        Transform::from_translation(base + Vec3::new(130.0, 0.0, 0.6)),
        HudEnemyHealthFill,
    ));

    // Controls legend text as a column on the left side
    let controls = " Controls:\n- Characters: 1-6 \n- Move: WASD / Arrows\n- Run: Shift\n- Jump: Space\n- Combat: Space / Enter\n- Restart: R\n- Quit: Q";
    commands.spawn((
        Text2d::new(controls.to_string()),
        TextFont { font_size: 16.0, ..Default::default() },
        // Increase contrast: use white text
        TextColor(Color::WHITE),
        TextLayout { justify: Justify::Left, ..Default::default() },
        // Position on the left side of the screen, below the top bars
        Transform::from_translation(Vec3::new(cam_pos.x - 360.0, cam_pos.y + 140.0, HUD_Z + 0.6)),
        HudControlsText,
    ));
}

// Follow the current camera so HUD sticks to the screen corners
pub fn position_hud_to_camera(
    cam_q: Query<&Transform, With<Camera2d>>,
    mut transforms: ParamSet<(
        Query<&'static mut Transform, (With<HudRoot>, Without<Camera2d>)>, 
        Query<&'static mut Transform, (With<HudHealthFill>, Without<HudRoot>, Without<Camera2d>)>, 
        Query<&'static mut Transform, (With<HudEnemyHealthFill>, Without<HudRoot>, Without<HudHealthFill>, Without<Camera2d>)>,
        Query<&'static mut Transform, (With<HudHealthBg>, Without<HudRoot>, Without<HudHealthFill>, Without<HudEnemyHealthFill>, Without<Camera2d>)>,
        Query<&'static mut Transform, (With<HudEnemyHealthBg>, Without<HudRoot>, Without<HudHealthFill>, Without<HudEnemyHealthFill>, Without<Camera2d>)>,
        Query<&'static mut Transform, (With<HudControlsText>, Without<HudRoot>, Without<HudHealthFill>, Without<HudEnemyHealthFill>, Without<Camera2d>)>,
    )>,
) {
    let Ok(cam) = cam_q.single() else { return; };
    let cam_pos = cam.translation;
    // Base for the top health bars
    let base = Vec3::new(cam_pos.x, cam_pos.y + 220.0, HUD_Z);

    // Move root and dependent elements together by computing offsets from base
    if let Ok(mut root_tf) = transforms.p0().single_mut() {
        root_tf.translation = base;
    }

    // Realign children relative to base (they are unparented for simplicity)
    if let Ok(mut tf) = transforms.p1().single_mut() { // player fill
        tf.translation = base + Vec3::new(-130.0, 0.0, 0.6);
    }
    if let Ok(mut tf) = transforms.p2().single_mut() { // enemy fill
        tf.translation = base + Vec3::new(130.0, 0.0, 0.6);
    }
    if let Ok(mut tf) = transforms.p3().single_mut() { // player bg
        tf.translation = base + Vec3::new(-130.0, 0.0, 0.5);
    }
    if let Ok(mut tf) = transforms.p4().single_mut() { // enemy bg
        tf.translation = base + Vec3::new(130.0, 0.0, 0.5);
    }
    if let Ok(mut tf) = transforms.p5().single_mut() { // controls (left column)
        // Position controls down and to the left from the top bars
        let left_pos = Vec3::new(cam_pos.x - 300.0, cam_pos.y + 140.0, HUD_Z + 0.6);
        tf.translation = left_pos;
    }
}

// Update HUD HP bars (player always, enemy only during active combat)
pub fn update_hud_health(
    player_q: Query<&CombatStats, With<crate::characters::movement::Player>>,
    enemy_q: Query<&CombatStats, (With<Enemy>, Without<crate::characters::movement::Player>)>,
    state: Res<CombatState>,
    mut sprite_sets: ParamSet<(
        Query<&'static mut Sprite, With<HudHealthFill>>,
        Query<&'static mut Sprite, (With<HudEnemyHealthFill>, Without<HudHealthFill>)>,
    )>,
) {
    let Ok(pstats) = player_q.single() else { return; };
    if let Ok(mut fill) = sprite_sets.p0().single_mut() {
        let ratio = (pstats.hp.max(0) as f32) / (pstats.max_hp.max(1) as f32);
        if let Some(size) = &mut fill.custom_size {
            size.x = 214.0 * ratio.clamp(0.0, 1.0);
        }
    }

    // Enemy bar reflects current combat target if combat is active; else hidden
    if let Some(active) = state.active.as_ref() {
        if let (Ok(estats), Ok(mut efill)) = (enemy_q.get(active.enemy), sprite_sets.p1().single_mut()) {
            let ratio = (estats.hp.max(0) as f32) / (estats.max_hp.max(1) as f32);
            if let Some(size) = &mut efill.custom_size {
                size.x = 214.0 * ratio.clamp(0.0, 1.0);
            }
        }
    } else if let Ok(mut efill) = sprite_sets.p1().single_mut() {
        if let Some(size) = &mut efill.custom_size { size.x = 0.0; }
    }
}
