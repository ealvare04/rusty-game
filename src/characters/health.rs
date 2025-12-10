// Small healing system based on collecting health pips
// inspired by Wizard101 and Pirate101 health orbs

use bevy::prelude::*;

use crate::characters::movement::Player;
use crate::characters::combat::{CombatStats, GameOutcome};
use crate::map::collision::{NonWalkable, Water, nonwalkable_half_extent, water_half_extent};
use crate::map::generate::{map_pixel_dimensions, TILE_SIZE};

// Small red pickups that restore player HP when collected
#[derive(Component)]
pub struct HealthPip;

// Track whether we've spawned pips already this run
#[derive(Resource, Default)]
pub struct HealthPipTracker {
    pub spawned: bool,
}

// Spawn a handful of health pips on valid ground (not on water)
pub fn spawn_health_pips_once(
    mut commands: Commands,
    mut tracker: ResMut<HealthPipTracker>,
    blocking_tiles: Query<&GlobalTransform, With<NonWalkable>>,
    water_tiles: Query<&GlobalTransform, With<Water>>,
    outcome: Res<GameOutcome>,
) {
    if tracker.spawned || !matches!(*outcome, GameOutcome::None) { return; }
    // Ensure terrain has spawned before placing pips so we can avoid water properly
    // If no water tiles are present yet, defer spawning to a later frame
    if water_tiles.iter().next().is_none() {
        return;
    }

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let map_size = map_pixel_dimensions();
    let half = map_size * 0.5;

    // helper: returns true if point collides a solid or water tile
    let would_collide = |point: Vec2| -> bool {
        let half_solid = nonwalkable_half_extent();
        for gt in blocking_tiles.iter() {
            let pos = gt.translation().truncate();
            let dx = (point.x - pos.x).abs();
            let dy = (point.y - pos.y).abs();
            if dx <= half_solid && dy <= half_solid { return true; }
        }
        let half_water = water_half_extent();
        for gt in water_tiles.iter() {
            let pos = gt.translation().truncate();
            let dx = (point.x - pos.x).abs();
            let dy = (point.y - pos.y).abs();
            // Use <= so pips never spawn on water or touching its bounds
            if dx <= half_water && dy <= half_water { return true; }
        }
        false
    };

    // spawn 6 pips
    let count = 6usize;
    for _ in 0..count {
        // find a valid location with several attempts
        let mut pos = None;
        for _ in 0..200 {
            let x = rng.gen_range((-half.x + TILE_SIZE)..(half.x - TILE_SIZE));
            let y = rng.gen_range((-half.y + TILE_SIZE)..(half.y - TILE_SIZE));
            let p = Vec2::new(x, y);
            if !would_collide(p) { pos = Some(p); break; }
        }
        let Some(p) = pos else { continue; };

        // Small red square sprite
        commands.spawn((
            Sprite { color: Color::srgb(0.9, 0.1, 0.1), custom_size: Some(Vec2::splat(8.0)), ..default() },
            Transform::from_translation(Vec3::new(p.x, p.y, 12.0)),
            HealthPip,
        ));
    }

    tracker.spawned = true;
}

// Collect pips when the player walks over them and heal
pub fn collect_health_pips(
    mut commands: Commands,
    mut player_q: Query<(&GlobalTransform, &mut CombatStats), With<Player>>,
    pips_q: Query<(Entity, &GlobalTransform), With<HealthPip>>,
    outcome: Res<GameOutcome>,
) {
    if !matches!(*outcome, GameOutcome::None) { return; }
    let Ok((p_tf, mut stats)) = player_q.single_mut() else { return; };
    let p = p_tf.translation().truncate();
    let radius = TILE_SIZE * 0.4;

    // Heal amount now scales with the player's max HP to stay relevant with large health pools.
    // Tweakable fraction: 15% of max HP, minimum of 1 HP.
    const PIP_HEAL_PERCENT: f32 = 0.15;

    for (e, gt) in pips_q.iter() {
        let d = gt.translation().truncate().distance(p);
        if d <= radius {
            // Heal a percentage of max HP, rounded to nearest int, at least 1
            let heal = ((stats.max_hp as f32) * PIP_HEAL_PERCENT).round() as i32;
            let heal = heal.max(1);
            stats.hp = (stats.hp + heal).min(stats.max_hp);
            commands.entity(e).despawn();
        }
    }
}
