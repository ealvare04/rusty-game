// from https://aibodh.com/posts/bevy-rust-game-development-chapter-3/
pub mod animation;
pub mod config;
pub mod movement;
pub mod spawn;

// added enemy NPCs, combat system, health pips, and a UI
pub mod npc;
pub mod combat;
pub mod health;
pub mod ui;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use config::CharactersList;

pub struct CharactersPlugin;

impl Plugin for CharactersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<CharactersList>::new(&["characters.ron"]))
            // from tutorial ch. 3
            .init_resource::<spawn::CurrentCharacterIndex>()

            // added Combat
            .init_resource::<combat::CombatState>()
            .init_resource::<combat::GameOutcome>()
            .init_resource::<combat::CombatLog>()
            .init_resource::<npc::EnemyTracker>()
            .init_resource::<health::HealthPipTracker>()

            // from tutorial ch. 3
            .add_systems(Startup, spawn::spawn_player)

            // added UI
            // Lightweight HUD systems in their own group to avoid exceeding tuple size limits
            .add_systems(Update, (
                ui::spawn_hud_once,
                ui::position_hud_to_camera,
                ui::update_hud_health,
            ))

            // added player stats, and attack animations,
            .add_systems(Update, (
                // from tutorial ch. 3
                spawn::initialize_player_character,
                spawn::switch_character,
                combat::sync_player_stats,
                movement::move_player,
                movement::update_jump_state,
                animation::animate_characters,
                animation::revert_attack_when_finished,
                animation::update_animation_flags,
                
                // pickups
                health::spawn_health_pips_once,
                health::collect_health_pips,
                
                // NPCs
                npc::spawn_enemies_once,
                npc::detect_player_proximity_start_combat,
                combat::combat_input_and_turns,
                
                // Combat UI systems
                combat::spawn_combat_ui_on_start,
                combat::update_combat_ui,
                combat::cleanup_combat_ui_on_end,
                // Outcome overlays and restart
                combat::show_outcome_overlay,
                combat::handle_restart_input,
                // Quit keybind
                combat::handle_quit_input,
            ))
            // Additional systems kept in separate tuple to avoid exceeding tuple size limits
            .add_systems(Update, (
                combat::handle_enemy_death_cleanup,
                combat::handle_player_death_outcome,
            ));
    }
}