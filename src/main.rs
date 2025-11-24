// import player and map
mod map;
mod player;

use bevy::{
    prelude::*,
    window::{Window, WindowPlugin, WindowResolution},
};

use bevy_procedural_tilemaps::prelude::*;

use crate::map::generate::{map_pixel_dimensions, setup_generator};
use crate::player::PlayerPlugin;

fn main(){
    
    // for map generation
    let map_size = map_pixel_dimensions();

    // creates game app
    App::new()
        // background color: WHITE
        .insert_resource(ClearColor(Color::WHITE))

        // loads rendering, input, audio, and other systems
        .add_plugins(
            // Bevy's AssetPlugin loads textures, audio, and other assets
            DefaultPlugins
                .set(AssetPlugin {
                    // assets are in 'src/assets'
                    file_path: "src/assets".into(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(map_size.x as u32, map_size.y as u32),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )

        .add_plugins(ProcGenSimplePlugin::<Cartesian3D, Sprite>::default())
        
        // registers startup function
        .add_systems(Startup, (setup_camera, setup_generator))
        // adds player plugin
        .add_plugins(PlayerPlugin)

        // gives control to Bevy's main loop,
        // which polls input, runs the games systems, updates the world, and renders a frame.
        // Loops until game is quit
        .run();
}

// sets up cameras, players, etc. before the first frame
fn setup_camera(mut commands: Commands){
    commands.spawn(Camera2d);    
}