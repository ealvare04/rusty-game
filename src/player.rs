use bevy::prelude::*;

// Atlas/Map constants
const TILE_SIZE: u32 = 64; //64x64 tiles
const WALK_FRAMES: usize = 9; //9 columns per walking row
const MOVE_SPEED: f32 = 140.0; // pixels per second
const ANIM_DT: f32 = 0.1; // seconds per frame (~10 fps)

// stores Player entities
// a component is a piece of data attached to an entity
#[derive(Component)]
struct Player;

// stores Player directions
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum Facing{
    Up,
    Left,
    Down,
    Right,
}

// Bevy Timer 
#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);


#[derive(Component)]
struct AnimationState {
    facing: Facing,
    moving: bool,
    was_moving: bool,
}

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // load spritesheet and build a grid layout
    // 64x64 tiles
    // 9 columns
    // 12 rows
    let texture = asset_server.load("player.png");
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(TILE_SIZE),
        WALK_FRAMES as u32,
        12, 
        None,
        None,
    ));

    // start facing down
    let facing = Facing::Down;
    let start_index = atlas_index_for(facing, 0);

    commands.spawn((
        Sprite::from_atlas_image(
            texture,
            TextureAtlas {
                layout,
                index: start_index,
            },
        ),
        Transform::from_translation(Vec3::ZERO),
        Player,
        AnimationState { facing, moving: false, was_moving: false },
        AnimationTimer(Timer::from_seconds(ANIM_DT, TimerMode::Repeating)),
    ));
}

// Res (or Resources) are pieces of game-wide information that arent't tied to any single entity
// general game information
fn move_player(
    // keyboard input 
    input: Res<ButtonInput<KeyCode>>,
    // time
    time: Res<Time>,
    // player's position
    mut player: Query<(&mut Transform, &mut AnimationState), With<Player>>,
) {
    let Ok((mut transform, mut anim)) = player.single_mut() else {
        return;
    };

    // starting position, Vec2 { x: 0.0, y: 0.0 }
    let mut direction = Vec2::ZERO;

    // Arrow(Position) are enums
    // asks Bevy for which button is held down
    if input.pressed(KeyCode::ArrowLeft){
        direction.x -= 1.0;
    }
    if input.pressed(KeyCode::ArrowRight){
        direction.x += 1.0;
    }
    if input.pressed(KeyCode::ArrowUp){
        direction.y += 1.0;
    }
    if input.pressed(KeyCode::ArrowDown){
        direction.y -= 1.0;
    }

    // stands still if nothing is pressed
    if direction != Vec2::ZERO {


        // normalize() converts the vector to length 1
        // so diagonal movement isn't faster than straight movement

        // time.delta_secs() returns the number of seconds since the previous frame (frame time)
        
        // multiplying them gives the distance the Player entity should travel this update
        let delta = direction.normalize() * MOVE_SPEED * time.delta_secs();

        // adds the calculated travel distance to the player's transform translation
        // to move the sprite on the screen
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
        anim.moving = true;

        //update facing
        if direction.x.abs() > direction.y.abs(){
            // positive x move -> moved right
            anim.facing = if direction.x > 0.0 { Facing::Right } else { Facing::Left }; 
        } else {
            // positive y move -> moved up
            anim.facing = if direction.y > 0.0 { Facing::Up } else { Facing::Down }; 
        }
    } else {
            anim.moving = false;
    }
}

fn animate_player(
    time: Res<Time>,
    mut query: Query<(&mut AnimationState, &mut AnimationTimer, &mut Sprite), With<Player>>,
) {
    // check the result and names the pieces we need
    // if Ok, the code binds anim, timer, and sprite to use later
    // else we exit
    let Ok((mut anim, mut timer, mut sprite)) = query.single_mut() else {
        return;
    };

    let atlas = match sprite.texture_atlas.as_mut() {
        Some(a) => a,
        None => return,
    };

    // compute target row and current position
    let target_row = row_zero_based(anim.facing);
    let mut current_col = atlas.index % WALK_FRAMES;
    let mut current_row = atlas.index / WALK_FRAMES;


    // if the facing changed 
    if current_row != target_row {
        atlas.index = row_start_index(anim.facing);
        current_col = 0;
        current_row = target_row;
        timer.reset();
    }

    let just_started = anim.moving && !anim.was_moving;
    let just_stopped = !anim.moving && anim.was_moving;

    if anim.moving {
        if just_started {
            // on movement start, immediately advance one frame for visible feedback
            let row_start = row_start_index(anim.facing);
            let next_col = (current_col + 1) % WALK_FRAMES;
            atlas.index = row_start + next_col;

            //restart the timer so the next advance uses a full interval
            timer.reset();
        } else {
            // continuous movement: advance based on timer cadence
            timer.tick(time.delta());
            if timer.just_finished() {
                let row_start = row_start_index(anim.facing);
                let next_col = (current_col + 1) % WALK_FRAMES;
                atlas.index = row_start + next_col;
            }
        }
    } else if just_stopped {
        // not moving: keep current frame to avoid snap. Reset timer on transition to idle
        timer.reset();
    }

    //update previous movement state
    anim.was_moving = anim.moving;
}

// returns the starting atlas index for the given facing row
fn row_start_index(facing: Facing) -> usize {
    row_zero_based(facing) * WALK_FRAMES
}

fn atlas_index_for(facing: Facing, frame_in_row: usize) -> usize {
    row_start_index(facing) + frame_in_row.min(WALK_FRAMES - 1)
}

// !!! i believe this sets the png for the player based on what position they are facing 
fn row_zero_based(facing: Facing) -> usize {
    match facing {
        Facing::Up => 8,
        Facing::Left => 9,
        Facing::Down=> 10,
        Facing::Right => 11,
    }
}

// PLAYER PLUGIN 
pub struct PlayerPlugin;

// apply Bevy's Plugin trait for PlayerPlugin
// Plugin needs a build function to register every system
// here it is implemented for the player-specific systems

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (move_player, animate_player));
    }
}