// characters/config.rs
// from https://aibodh.com/posts/bevy-rust-game-development-chapter-3/
// used to configure character animations and textures
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// HashMap to store AnimationType as a key
// Serialize and Deserialize to turn structs into .ron text
// added Attack and Death animation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnimationType {
    Walk,
    Run,
    Jump,
    Attack,
    Death,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationDefinition {
    pub start_row: usize,
    pub frame_count: usize,
    pub frame_time: f32,
    pub directional: bool, // true = 4 rows (one per direction), false = 1 row
}

// Bundle character attributes, sprite metadata, and animation map
// seen in characters.ron
// added attack_damage
#[derive(Component, Asset, TypePath, Debug, Clone, Serialize, Deserialize)]
pub struct CharacterEntry {
    pub name: String,
    pub max_health: f32,
    // Base attack damage used for enemies of this character type
    // (players currently use internal defaults; this field mainly drives enemy damage)
    pub attack_damage: f32,
    pub base_move_speed: f32,
    pub run_speed_multiplier: f32,
    pub texture_path: String,
    pub tile_size: u32,
    pub atlas_columns: usize,
    pub animations: HashMap<AnimationType, AnimationDefinition>,
}

impl CharacterEntry {
    // inspects every animation definition to figure out how many rows the texture atlas needs
    pub fn calculate_max_animation_row(&self) -> usize {
        self.animations
            .values()
            .map(|def| if def.directional { def.start_row + 3 } else { def.start_row })
            .max()
            .unwrap_or(0)
    }
}

// Asset - loadable assets,
// TypePath - gives Bevy a unique name for types to know what asset to use
#[derive(Asset, TypePath, Debug, Clone, Serialize, Deserialize)]
pub struct CharactersList {
    pub characters: Vec<CharacterEntry>,
}
