// Movement System

use bevy::prelude::*;
use crate::map::collision::NonWalkable;
use crate::map::generate::TILE_SIZE;
use crate::characters::animation::*;
use crate::characters::config::{CharacterEntry, AnimationType};

/// Read directional input and return a direction vector
fn read_movement_input(input: &ButtonInput<KeyCode>) -> Vec2 {
    const MOVEMENT_KEYS: [(KeyCode, Vec2); 4] = [
        (KeyCode::ArrowLeft, Vec2::NEG_X),
        (KeyCode::ArrowRight, Vec2::X),
        (KeyCode::ArrowUp, Vec2::Y),
        (KeyCode::ArrowDown, Vec2::NEG_Y),
    ];

    MOVEMENT_KEYS.iter()
        .filter(|(key, _)| input.pressed(*key))
        .map(|(_, dir)| *dir)
        .sum()
}

/// Calculate movement speed based on character config and running state
fn calculate_movement_speed(character: &CharacterEntry, is_running: bool) -> f32 {
    if is_running {
        character.base_move_speed * character.run_speed_multiplier
    } else {
        character.base_move_speed
    }
}

// THE PLAYER MARKER
/// Marker component for the player entity
#[derive(Component)]
pub struct Player;

/// Handle player movement input and update transform/animation
pub fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(
        &mut Transform,
        &mut AnimationController,
        &mut AnimationState,
        &CharacterEntry,
    ), With<Player>>,
    blocking_tiles: Query<&GlobalTransform, With<NonWalkable>>,
) {
    let Ok((mut transform, mut animated, mut state, character)) = query.single_mut() else {
        return;
    };

    let direction = read_movement_input(&input);

    // Check for jump input (space key)
    if input.just_pressed(KeyCode::Space) {
        state.is_jumping = true;
        animated.current_animation = AnimationType::Jump;
    }

    // Check if running
    let is_running = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);

    // Handle movement
    if direction != Vec2::ZERO {
        let move_speed = calculate_movement_speed(character, is_running);
        let delta = direction.normalize() * move_speed * time.delta_secs();

        // Collision-aware movement: resolve per-axis against NonWalkable tiles
        let mut new_pos = transform.translation;

        // Attempt X movement
        if delta.x != 0.0 {
            let candidate = Vec2::new(new_pos.x + delta.x, new_pos.y);
            if !would_collide_point(candidate, &blocking_tiles) {
                new_pos.x += delta.x;
            }
        }

        // Attempt Y movement
        if delta.y != 0.0 {
            let candidate = Vec2::new(new_pos.x, new_pos.y + delta.y);
            if !would_collide_point(candidate, &blocking_tiles) {
                new_pos.y += delta.y;
            }
        }

        transform.translation = new_pos;

        animated.facing = Facing::from_direction(direction);

        // Only update animation if not jumping
        if !state.is_jumping {
            state.is_moving = true;
            animated.current_animation = if is_running {
                AnimationType::Run
            } else {
                AnimationType::Walk
            };
        }
    } else if !state.is_jumping {
        state.is_moving = false;
        animated.current_animation = AnimationType::Walk;
    }
}

// Handling Jump Completion
// jump and return to idle
/// Monitor jump animation completion and reset state
pub fn update_jump_state(
    mut query: Query<(
        &mut AnimationController,
        &mut AnimationState,
        &AnimationTimer,
        &Sprite,
        &CharacterEntry,
    ), With<Player>>,
) {
    for (mut animated, mut state, timer, sprite, config) in query.iter_mut() {
        if !state.is_jumping {
            continue;
        }

        let Some(atlas) = sprite.texture_atlas.as_ref() else {
            continue;
        };

        let Some(clip) = animated.get_clip(config) else {
            continue;
        };

        // Check if jump animation has completed
        if clip.is_complete(atlas.index, timer.just_finished()) {
            state.is_jumping = false;
            animated.current_animation = AnimationType::Walk;
        }
    }
}

/// Check if a point would overlap any NonWalkable tile's AABB
fn would_collide_point(point: Vec2, blocking_tiles: &Query<&GlobalTransform, With<NonWalkable>>) -> bool {
    let half = TILE_SIZE * 0.5;
    for gt in blocking_tiles.iter() {
        let pos = gt.translation().truncate();
        let dx = (point.x - pos.x).abs();
        let dy = (point.y - pos.y).abs();
        if dx <= half && dy <= half {
            return true;
        }
    }
    false
}
