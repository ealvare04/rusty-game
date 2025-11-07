use bevy::prelude::*;

use crate::player::PlayerPlugin;

// import player.rs
mod player;

fn main(){
    // creates game app
    App::new()
        // background color: WHITE
        .insert_resource(ClearColor(Color::WHITE))

        // loads rendering, input, audio, and other systems
        .add_plugins(
            // Bevy's AssetPlugin loads textures, audio, and other assets
            DefaultPlugins.set(AssetPlugin {
                // assets are in 'src/assets'
                file_path: "src/assets".into(),
                ..default()
            }),
        )
        
        // registers startup function
        .add_systems(Startup, setup)
        // adds player plugin
        .add_plugins(PlayerPlugin)

        // gives control to Bevy's main loop,
        // which polls input, runs the games systems, updates the world, and renders a frame.
        // Loops until game is quit
        .run();
}

// sets up cameras, players, etc. before the first frame
fn setup(mut commands: Commands){
    commands.spawn(Camera2d);    
}