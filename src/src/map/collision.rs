use bevy::prelude::*;

/// Marker component for tiles that should block player movement
#[derive(Component)]
pub struct NonWalkable;

/// function to insert NonWalkable during asset spawn
pub fn insert_blocking(ec: &mut EntityCommands) {
    ec.insert(NonWalkable);
}
