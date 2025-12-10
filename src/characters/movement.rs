// Movement System
// from https://aibodh.com/posts/bevy-rust-game-development-chapter-3/
// added collision handling, idle when combat starts, and player death handling

use bevy::prelude::*;
use crate::map::collision::{NonWalkable, Water, nonwalkable_half_extent, water_half_extent};
use crate::map::generate::TILE_SIZE;
use crate::characters::animation::*;
use crate::characters::combat::{CombatState, GameOutcome};
use crate::characters::config::{CharacterEntry, AnimationType};

/// Read directional input and return a direction vector
fn read_movement_input(input: &ButtonInput<KeyCode>) -> Vec2 {
    const MOVEMENT_KEYS: [(KeyCode, Vec2); 8] = [
        /* Arrow keys controls */
        (KeyCode::ArrowLeft, Vec2::NEG_X),
        (KeyCode::ArrowRight, Vec2::X),
        (KeyCode::ArrowUp, Vec2::Y),
        (KeyCode::ArrowDown, Vec2::NEG_Y),

        /* WASD controls */
        (KeyCode::KeyW, Vec2::Y),
        (KeyCode::KeyA, Vec2::NEG_X),
        (KeyCode::KeyS, Vec2::NEG_Y),
        (KeyCode::KeyD, Vec2::X),
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
    combat: Res<CombatState>,
    outcome: Res<GameOutcome>,
    mut query: Query<(
        &mut Transform,
        &mut AnimationController,
        &mut AnimationState,
        &CharacterEntry,
    ), With<Player>>,
    blocking_tiles: Query<&GlobalTransform, With<NonWalkable>>,
    water_tiles: Query<&GlobalTransform, With<Water>>,
) {
    let Ok((mut transform, mut animated, mut state, character)) = query.single_mut() else {
        return;
    };

    // If currently playing a Death animation, disable movement entirely
    if matches!(animated.current_animation, AnimationType::Death) {
        state.is_moving = false;
        return;
    }

    // When combat starts (or an outcome overlay is showing), force player to idle
    // and disable further movement processing.
    if combat.active.is_some() || !matches!(*outcome, GameOutcome::None) {
        state.is_moving = false;
        // Do not override Attack/Death animations while they are playing
        if !state.is_jumping && !matches!(animated.current_animation, AnimationType::Death | AnimationType::Attack) {
            animated.current_animation = AnimationType::Walk; // use Walk's idle frame as idle
        }
        return;
    }

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

        // Collision-aware movement with swept sub-steps to avoid tunneling
        let mut new_pos = transform.translation;

        // Determine number of sub-steps based on the maximum component
        // Keep each step relatively small vs tile size to avoid skipping over thin obstacles
        let max_component = delta.x.abs().max(delta.y.abs());
        let max_step_len = TILE_SIZE * 0.20; // at most 20% of a tile per sub-step
        let steps = if max_component > 0.0 {
            (max_component / max_step_len).ceil().clamp(1.0, 8.0) as u32 // clamp to avoid perf issues
        } else { 1 };

        let step = Vec2::new(delta.x / steps as f32, delta.y / steps as f32);

        for _ in 0..steps {
            // Attempt X movement for this sub-step
            if step.x != 0.0 {
                let candidate = Vec2::new(new_pos.x + step.x, new_pos.y);
                if !would_collide_point(candidate, &blocking_tiles, &water_tiles) {
                    new_pos.x += step.x;
                }
            }

            // Attempt Y movement for this sub-step
            if step.y != 0.0 {
                let candidate = Vec2::new(new_pos.x, new_pos.y + step.y);
                if !would_collide_point(candidate, &blocking_tiles, &water_tiles) {
                    new_pos.y += step.y;
                }
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

/// Check if a point would overlap any solid (NonWalkable) or Water tile's AABB
fn would_collide_point(
    point: Vec2,
    solids: &Query<&GlobalTransform, With<NonWalkable>>,
    waters: &Query<&GlobalTransform, With<Water>>,
) -> bool {
    // Solids: full half extent, inclusive test
    let solid_half = nonwalkable_half_extent();
    for gt in solids.iter() {
        let pos = gt.translation().truncate();
        let dx = (point.x - pos.x).abs();
        let dy = (point.y - pos.y).abs();
        if dx <= solid_half && dy <= solid_half {
            return true;
        }
    }

    // Water: slightly smaller half extent and strict inequality so edges on grass aren't blocked
    let water_half = water_half_extent();
    for gt in waters.iter() {
        let pos = gt.translation().truncate();
        let dx = (point.x - pos.x).abs();
        let dy = (point.y - pos.y).abs();
        if dx < water_half && dy < water_half {
            return true;
        }
    }

    false
}
