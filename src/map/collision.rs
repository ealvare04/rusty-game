// custom collision system

use bevy::prelude::*;
use crate::map::generate::TILE_SIZE;

/// Component to mark tiles that should block player movement
#[derive(Component)]
pub struct NonWalkable;

/// function to insert NonWalkable component during asset spawn
pub fn insert_blocking(ec: &mut EntityCommands) {
    ec.insert(NonWalkable);
}

/// Component for water tiles that should block the player from entering the water,
/// but allow movement along the edge
#[derive(Component)]
pub struct Water;

/// function to insert Water during asset spawn
pub fn insert_water_blocking(ec: &mut EntityCommands) {
    ec.insert(Water);
}

/// Returns the half-extent used for NonWalkable (water) collision AABBs.
/// We use the full half-tile extent so solid tiles (trees, rocks, etc.) are fully solid
/// and do not leave gaps that the player can slip through.
pub fn nonwalkable_half_extent() -> f32 {
    // Use full half tile so solids have solid, gap-free collision.
    TILE_SIZE * 0.5
}

/// Returns the half-extent used for Water collision AABBs.
/// Slightly less than half a tile so walking on adjacent ground is not blocked
/// by an exact tile boundary overlap, but still prevents entering water.
pub fn water_half_extent() -> f32 {
    TILE_SIZE * 0.48
}
