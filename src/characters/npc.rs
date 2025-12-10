// NPC Enemy spawning system

use bevy::prelude::*;

use crate::characters::animation::*;
use crate::characters::combat::{ActiveCombat, CombatState, CombatStats, GameOutcome};
use crate::characters::config::{CharacterEntry, CharactersList};
use crate::characters::movement::Player;
use crate::characters::spawn::CharactersListResource;

use crate::map::generate::{map_pixel_dimensions, TILE_SIZE};
use crate::map::collision::{NonWalkable, Water, nonwalkable_half_extent, water_half_extent};

const ENEMY_SCALE: f32 = 0.8;
const ENEMY_Z: f32 = 15.0;
const ENEMIES_TO_SPAWN: usize = 3;

// Public marker for enemy entities
#[derive(Component)]
pub struct Enemy;

// Tracks enemy spawn state and remaining count
#[derive(Resource, Default)]
pub struct EnemyTracker {
    pub spawned: bool,
    pub alive: usize,
}

fn easy_enemy_stats(entry: &CharacterEntry) -> CombatStats {
    // Build enemy stats directly from the RON entry.
    // Health and attack come from the config
    // keep low defense to make fights readable.
    let max_hp_i = entry.max_health.max(1.0).round() as i32;
    CombatStats {
        max_hp: max_hp_i,
        hp: max_hp_i,
        attack: entry.attack_damage.max(1.0).round() as i32,
        defense: 0,
        // Keep simple defaults for crit/evade
        crit_chance: 0.10,
        evade_chance: 0.10,
    }
}

pub fn spawn_enemies_once(
    mut commands: Commands,
    characters_lists: Res<Assets<CharactersList>>,
    characters_list_res: Option<Res<CharactersListResource>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
    mut tracker: ResMut<EnemyTracker>,
    blocking_tiles: Query<&GlobalTransform, With<NonWalkable>>,
    water_tiles: Query<&GlobalTransform, With<Water>>,
) {
    if tracker.spawned { return; }
    let Some(list_res) = characters_list_res else { return; };
    let Some(list) = characters_lists.get(&list_res.handle) else { return; };
    if list.characters.is_empty() { return; }

    let map_size = map_pixel_dimensions();
    let half = map_size * 0.5;

    // rng using rand crate
    use rand::Rng;
    let mut rng = rand::rng();

    // helper to test collision with solid or water tiles
    let would_collide = |point: Vec2| -> bool {
        let solid_half = nonwalkable_half_extent();
        for gt in blocking_tiles.iter() {
            let pos = gt.translation().truncate();
            let dx = (point.x - pos.x).abs();
            let dy = (point.y - pos.y).abs();
            if dx <= solid_half && dy <= solid_half { return true; }
        }
        let water_half = water_half_extent();
        for gt in water_tiles.iter() {
            let pos = gt.translation().truncate();
            let dx = (point.x - pos.x).abs();
            let dy = (point.y - pos.y).abs();
            if dx < water_half && dy < water_half { return true; }
        }
        false
    };

    for _ in 0..ENEMIES_TO_SPAWN {
        // Pick a random character entry for variety
        let idx = if list.characters.len() > 1 {
            rng.random_range(0..list.characters.len())
        } else { 0 };
        let enemy_entry: &CharacterEntry = &list.characters[idx];

        // Prepare atlas and texture for this enemy type
        let layout = {
            let max_row = enemy_entry.calculate_max_animation_row();
            atlas_layouts.add(TextureAtlasLayout::from_grid(
                UVec2::splat(enemy_entry.tile_size),
                enemy_entry.atlas_columns as u32,
                (max_row + 1) as u32,
                None,
                None,
            ))
        };
        let texture: Handle<Image> = asset_server.load(&enemy_entry.texture_path);

        // sample a valid ground position (not on water/non-walkable)
        let mut pos = Vec2::ZERO;
        for _attempt in 0..200 {
            let x = rng.random_range((-half.x + TILE_SIZE)..(half.x - TILE_SIZE));
            let y = rng.random_range((-half.y + TILE_SIZE)..(half.y - TILE_SIZE));
            let candidate = Vec2::new(x, y);
            if !would_collide(candidate) {
                pos = candidate;
                break;
            }
        }
        let sprite = Sprite::from_atlas_image(
            texture.clone(),
            TextureAtlas { layout: layout.clone(), index: 0 },
        );

        commands.spawn((
            Enemy,
            easy_enemy_stats(enemy_entry),
            enemy_entry.clone(),
            AnimationController::default(),
            AnimationState::default(),
            AnimationTimer(Timer::from_seconds(DEFAULT_ANIMATION_FRAME_TIME, TimerMode::Repeating)),
            Transform::from_translation(Vec3::new(pos.x, pos.y, ENEMY_Z)).with_scale(Vec3::splat(ENEMY_SCALE)),
            sprite,
        ));
    }
    tracker.alive = ENEMIES_TO_SPAWN;
    tracker.spawned = true;
}

// Start combat when the player is close to an enemy
pub fn detect_player_proximity_start_combat(
    mut state: ResMut<CombatState>,
    outcome: Res<GameOutcome>,
    player_q: Query<(Entity, &GlobalTransform), With<Player>>,
    enemies_q: Query<(Entity, &GlobalTransform), With<Enemy>>,
) {
    if state.active.is_some() { return; }
    if !matches!(*outcome, GameOutcome::None) { return; }
    let Ok((player_e, p_tf)) = player_q.single() else { return; };
    let p = p_tf.translation().truncate();

    let trigger_dist = TILE_SIZE * 0.75;
    for (enemy_e, e_tf) in enemies_q.iter() {
        let d = e_tf.translation().truncate().distance(p);
        if d < trigger_dist {
            state.active = Some(ActiveCombat { player: player_e, enemy: enemy_e, players_turn: true });
            break;
        }
    }
}
