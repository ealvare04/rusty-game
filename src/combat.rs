// added Combat system

use bevy::prelude::*;

use crate::characters::movement::Player;
use crate::characters::config::{CharacterEntry, AnimationType};
use crate::characters::npc::{EnemyTracker, Enemy};
use crate::characters::animation::Facing;

// Enemy component is defined in npc.rs

#[derive(Component, Debug, Clone, Copy)]
pub struct CombatStats {
    pub max_hp: i32,
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub crit_chance: f32,  // 0.0..1.0
    pub evade_chance: f32, // 0.0..1.0
}
impl CombatStats {
}

#[derive(Resource, Default)]
pub struct CombatState {
    pub active: Option<ActiveCombat>,
}

#[derive(Debug)]
pub struct ActiveCombat {
    pub player: Entity,
    pub enemy: Entity,
    pub players_turn: bool,
}

// Overall game outcome
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOutcome {
    None,
    GameOver,
    GameWon,
}

impl Default for GameOutcome {
    fn default() -> Self { GameOutcome::None }
}

// Simple in-window combat UI and outcome overlays using Sprites
#[derive(Component)]
pub struct CombatUiRoot;

#[derive(Component)]
pub struct PlayerHpBar;

#[derive(Component)]
pub struct EnemyHpBar;

#[derive(Component)]
pub struct OutcomeUi;

// Combat turn/result log and bottom-of-screen combat UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackResult { Hit, Miss }

#[derive(Resource, Default, Debug, Clone)]
pub struct CombatLog {
    pub last_player: Option<AttackResult>,
    pub last_enemy: Option<AttackResult>,
    // Latest console-style message to mirror in UI (e.g., "Player hits enemy for 12 (enemy hp 34)")
    pub last_msg: Option<String>,
}

#[derive(Component)]
pub struct CombatUiLeftText;  // Player column

#[derive(Component)]
pub struct CombatUiRightText; // Enemy column

// Center bottom combat alerts (mirror console info! logs)
#[derive(Component)]
pub struct CombatUiLogText;

// Add player stats based on the RON config
pub fn sync_player_stats(
    mut commands: Commands,
    q: Query<(Entity, Option<&CharacterEntry>), (With<Player>, Without<CombatStats>)>,
) {
    for (e, config_opt) in q.iter() {
        let max = config_opt.map(|c| c.max_health).unwrap_or(30.0);
        // Inline the previous CombatStats::new_basic defaults here
        let max_hp_i = max.max(1.0).round() as i32;
        let mut stats = CombatStats {
            max_hp: max_hp_i,
            hp: max_hp_i,
            attack: 6,
            defense: 2,
            crit_chance: 0.15,
            evade_chance: 0.1,
        };
        // If we know the character entry, use its attack_damage for the player as well
        if let Some(cfg) = config_opt {
            stats.attack = cfg.attack_damage.max(1.0).round() as i32;
        }
        commands.entity(e).insert(stats);

        info!("Player stats synced");
    }
}

// Main combat driver: press Space/Enter to advance turns. Probability-based hit/damage.
pub fn combat_input_and_turns(
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CombatState>,
    mut player_q: Query<&mut CombatStats, With<Player>>, // only stats are mutated here
    mut enemy_q: Query<(&mut CombatStats, &GlobalTransform), (With<Enemy>, Without<Player>)>,
    mut anim_sets: ParamSet<(
        Query<(&GlobalTransform, &mut crate::characters::animation::AnimationController, &mut crate::characters::animation::AnimationState), With<Player>>,
        Query<(&GlobalTransform, &mut crate::characters::animation::AnimationController, &mut crate::characters::animation::AnimationState), (With<Enemy>, Without<Player>)>,
        Query<(&crate::characters::animation::AnimationController, &crate::characters::animation::AnimationTimer, &Sprite, &crate::characters::config::CharacterEntry), With<Player>>,
        Query<(&crate::characters::animation::AnimationController, &crate::characters::animation::AnimationTimer, &Sprite, &crate::characters::config::CharacterEntry), (With<Enemy>, Without<Player>)>,
    )>,

    /*
    mut outcome: ResMut<GameOutcome>,
    mut enemy_tracker: ResMut<EnemyTracker>,
    mut commands: Commands,
     */

    mut clog: ResMut<CombatLog>,
) {
    // Damage scaling to make combat resolve faster while keeping balance fair.
    // Slightly favor the player and slightly reduce enemy damage as requested.
    const PLAYER_DAMAGE_MULTIPLIER: f32 = 2.2; // +10% vs previous 2.0
    const ENEMY_DAMAGE_MULTIPLIER: f32 = 1.8;  // -10% vs previous 2.0

    let Some(active) = state.active.as_mut() else { return; };

    let proceed = input.just_pressed(KeyCode::Space) || input.just_pressed(KeyCode::Enter);
    if !proceed { return; }

    // If an attack animation is currently running for either actor, wait until it finishes
    // to avoid skipping the animation by advancing the turn too quickly.
    // This also prevents spamming Space from queueing up multiple turns.
    let mut attack_in_progress = false;
    if let Ok((controller, timer, sprite, config)) = anim_sets.p2().get(active.player) {
        if matches!(controller.current_animation, AnimationType::Attack) {
            if let Some(atlas) = sprite.texture_atlas.as_ref() {
                if let Some(clip) = controller.get_clip(config) {
                    if !clip.is_complete(atlas.index, timer.0.is_finished()) {
                        attack_in_progress = true;
                    }
                }
            }
        }
    }
    if let Ok((controller, timer, sprite, config)) = anim_sets.p3().get(active.enemy) {
        if matches!(controller.current_animation, AnimationType::Attack) {
            if let Some(atlas) = sprite.texture_atlas.as_ref() {
                if let Some(clip) = controller.get_clip(config) {
                    if !clip.is_complete(atlas.index, timer.0.is_finished()) {
                        attack_in_progress = true;
                    }
                }
            }
        }
    }
    if attack_in_progress { return; }

    // Retrieve entities
    let mut p_stats = match player_q.get_mut(active.player) {
        Ok(v) => v,
        Err(_) => { state.active = None; return; }
    };
    let (mut e_stats, e_gtf) = match enemy_q.get_mut(active.enemy) {
        Ok(v) => v,
        Err(_) => { state.active = None; return; }
    };

    use rand::Rng;
    let mut rng = rand::rng();
    let mut rand01 = || rng.random::<f32>();

    // Turn resolution
    if active.players_turn {
        // Player attacks enemy
        let hit_chance: f32 = (0.85f32 - e_stats.evade_chance).clamp(0.1, 0.95);
        if rand01() < hit_chance {
            // Base damage with crit, then apply player-specific multiplier
            // max clamps damage at 1 to avoid 0 damage
            let mut dmg_f = ((p_stats.attack - e_stats.defense).max(1)) as f32;
            if rand01() < p_stats.crit_chance { dmg_f *= 2.0; }
            let dmg = (dmg_f * PLAYER_DAMAGE_MULTIPLIER).round().max(1.0) as i32;
            e_stats.hp -= dmg;

            info!("Player hits enemy for {} (enemy hp {})", dmg, e_stats.hp);

            clog.last_player = Some(AttackResult::Hit);
            clog.last_msg = Some(format!("Player hits enemy for {} (enemy hp {})", dmg, e_stats.hp));
        } else {
            info!("Player missed!");

            clog.last_player = Some(AttackResult::Miss);
            clog.last_msg = Some("Player missed!".to_string());
        }
        // Trigger player attack animation facing the enemy
        if let Ok((p_gtf, mut controller, mut astate)) = anim_sets.p0().get_mut(active.player) {
            let dir = (e_gtf.translation().truncate() - p_gtf.translation().truncate()).normalize_or_zero();
            controller.facing = Facing::from_direction(dir);
            controller.current_animation = AnimationType::Attack;
            astate.is_moving = false;
        }

    } else {
        // Enemy attacks player
        let hit_chance: f32 = (0.75f32 - p_stats.evade_chance).clamp(0.1, 0.95);
        if rand01() < hit_chance {
            // Base damage with crit, then apply enemy-specific multiplier
            // max clamps damage at 1 to avoid 0 damage
            let mut dmg_f = ((e_stats.attack - p_stats.defense).max(1)) as f32;
            if rand01() < e_stats.crit_chance { dmg_f *= 2.0; }
            let dmg = (dmg_f * ENEMY_DAMAGE_MULTIPLIER).round().max(1.0) as i32;
            p_stats.hp -= dmg;


            info!("Enemy hits player for {} (player hp {})", dmg, p_stats.hp);

            clog.last_enemy = Some(AttackResult::Hit);
            clog.last_msg = Some(format!("Enemy hits player for {} (player hp {})", dmg, p_stats.hp));
        } else {
            info!("Enemy missed!");

            clog.last_enemy = Some(AttackResult::Miss);
            clog.last_msg = Some("Enemy missed!".to_string());
        }
        // Trigger enemy attack animation facing the player
        // Borrow the player query first to read the position, then drop it before borrowing enemy mutably.
        let p_pos = if let Ok((p_gtf, _, _)) = anim_sets.p0().get_mut(active.player) {
            Some(p_gtf.translation().truncate())
        } else { None };

        if let (Some(p_pos), Ok((e_gtf2, mut controller, mut astate))) = (p_pos, anim_sets.p1().get_mut(active.enemy)) {
            let dir = (p_pos - e_gtf2.translation().truncate()).normalize_or_zero();
            controller.facing = Facing::from_direction(dir);
            controller.current_animation = AnimationType::Attack;
            astate.is_moving = false;
        }
    }

    // Check outcomes
    if e_stats.hp <= 0 {
        // Trigger enemy death animation and end combat; cleanup will occur after animation finishes
        if let Ok((_, mut ctrl, mut astate)) = anim_sets.p1().get_mut(active.enemy) {
            ctrl.current_animation = AnimationType::Death;
            astate.is_moving = false;
        }
        state.active = None;
        return;
    }

    if p_stats.hp <= 0 {
        // Trigger player death animation and end combat; outcome will be shown after animation finishes
        if let Ok((_p_gtf, mut ctrl, mut astate)) = anim_sets.p0().get_mut(active.player) {
            ctrl.current_animation = AnimationType::Death;
            astate.is_moving = false;
        }
        state.active = None;
        return;
    }

    // Switch turns
    active.players_turn = !active.players_turn;
}

// Despawn enemies only after their Death animation is finished and update outcome if needed
pub fn handle_enemy_death_cleanup(
    mut commands: Commands,
    mut enemy_tracker: ResMut<EnemyTracker>,
    mut outcome: ResMut<GameOutcome>,
    query: Query<(Entity, &crate::characters::animation::AnimationController, &crate::characters::animation::AnimationTimer, &Sprite, &crate::characters::config::CharacterEntry), With<Enemy>>,
) {
    use crate::characters::config::AnimationType;
    use crate::characters::animation::AnimationClip;

    for (entity, controller, timer, sprite, config) in query.iter() {
        if !matches!(controller.current_animation, AnimationType::Death) { continue; }
        let Some(atlas) = sprite.texture_atlas.as_ref() else { continue; };
        let Some(def) = config.animations.get(&AnimationType::Death) else { continue; };
        let row = if def.directional { def.start_row } else { def.start_row };
        let clip = AnimationClip::new(row, def.frame_count, config.atlas_columns);
        if clip.is_complete(atlas.index, timer.0.is_finished()) {
            // Death finished: despawn and adjust counts
            commands.entity(entity).despawn();
            if enemy_tracker.alive > 0 { enemy_tracker.alive -= 1; }
            if enemy_tracker.alive == 0 {
                *outcome = GameOutcome::GameWon;
            }
        }
    }
}

// After player Death animation completes, show Game Over
pub fn handle_player_death_outcome(
    mut outcome: ResMut<GameOutcome>,
    query: Query<(&crate::characters::animation::AnimationController, &crate::characters::animation::AnimationTimer, &Sprite, &crate::characters::config::CharacterEntry), With<Player>>,
) {
    use crate::characters::config::AnimationType;
    use crate::characters::animation::AnimationClip;
    if !matches!(*outcome, GameOutcome::None) { return; }

    let Ok((controller, timer, sprite, config)) = query.single() else { return; };

    if !matches!(controller.current_animation, AnimationType::Death) { return; }

    let Some(atlas) = sprite.texture_atlas.as_ref() else { return; };
    let Some(def) = config.animations.get(&AnimationType::Death) else { return; };
    let row = if def.directional { def.start_row } else { def.start_row };
    let clip = AnimationClip::new(row, def.frame_count, config.atlas_columns);

    if clip.is_complete(atlas.index, timer.0.is_finished()) {
        *outcome = GameOutcome::GameOver;
    }
}

// Spawn a simple overlay text when combat starts
pub fn spawn_combat_ui_on_start(
    mut commands: Commands,
    state: Res<CombatState>,
    existing: Query<Entity, With<CombatUiRoot>>,
    cam_q: Query<&Transform, With<Camera2d>>,
    mut clog: ResMut<CombatLog>,
) {
    if existing.iter().next().is_some() { return; }
    let Some(_active) = state.active.as_ref() else { return; };
    let cam_pos = cam_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    let base = Vec3::new(cam_pos.x, cam_pos.y - 220.0, 60.0);

    // Reset combat log on spawn
    clog.last_player = None;
    clog.last_enemy = None;
    clog.last_msg = None;

    // Root marker
    commands.spawn((Transform::from_translation(base), CombatUiRoot));

    // Left column: Player
    commands.spawn((
        Text2d::new("Player Turn\nLast: -"),
        TextFont { font_size: 18.0, ..Default::default() },
        // Increase contrast: use white text
        TextColor(Color::WHITE),
        TextLayout { justify: Justify::Left, ..Default::default() },
        Transform::from_translation(base + Vec3::new(-250.0, 0.0, 0.1)),
        CombatUiLeftText,
    ));

    // Right column: Enemy
    commands.spawn((
        Text2d::new("Enemy Turn\nLast: -"),
        TextFont { font_size: 18.0, ..Default::default() },
        // Increase contrast: use white text
        TextColor(Color::WHITE),
        TextLayout { justify: Justify::Right, ..Default::default() },
        Transform::from_translation(base + Vec3::new(250.0, 0.0, 0.1)),
        CombatUiRightText,
    ));

    // Center column: latest combat alert mirroring console log
    commands.spawn((
        Text2d::new("-"),
        TextFont { font_size: 18.0, ..Default::default() },
        TextColor(Color::WHITE),
        TextLayout { justify: Justify::Center, ..Default::default() },
        Transform::from_translation(base + Vec3::new(0.0, -24.0, 0.1)),
        CombatUiLogText,
    ));
}

// Update the combat UI each frame: position and text
pub fn update_combat_ui(
    state: Res<CombatState>,
    cam_q: Query<&Transform, With<Camera2d>>,
    clog: Res<CombatLog>,
    mut texts: Query<(
        &mut Transform,
        &mut Text2d,
        Option<&CombatUiLeftText>,
        Option<&CombatUiRightText>,
        Option<&CombatUiLogText>,
    ), Without<Camera2d>>,
) {
    if state.active.is_none() { return; }
    let Ok(cam) = cam_q.single() else { return; };
    let cam_pos = cam.translation;
    let base = Vec3::new(cam_pos.x, cam_pos.y - 220.0, 60.0);

    for (mut tf, mut text, is_left, is_right, is_log) in texts.iter_mut() {
        if is_left.is_some() {
            tf.translation = base + Vec3::new(-250.0, 0.0, 0.1);
            let turn = if state.active.as_ref().map(|a| a.players_turn).unwrap_or(false) { "Player Turn" } else { "" };
            let last = match clog.last_player {
                Some(AttackResult::Hit) => "Hit",
                Some(AttackResult::Miss) => "Miss",
                None => "-" };
            text.0 = format!("{}\nLast: {}", turn, last);

        } else if is_right.is_some() {
            tf.translation = base + Vec3::new(250.0, 0.0, 0.1);
            let turn = if state.active.as_ref().map(|a| a.players_turn).unwrap_or(true) { "" } else { "Enemy Turn" };
            let last = match clog.last_enemy {
                Some(AttackResult::Hit) => "Hit",
                Some(AttackResult::Miss) => "Miss",
                None => "-" };
            text.0 = format!("{}\nLast: {}", turn, last);

        } else if is_log.is_some() {
            tf.translation = base + Vec3::new(0.0, -24.0, 0.1);
            let msg = clog.last_msg.as_deref().unwrap_or("-");
            text.0 = msg.to_string();
        }
    }
}

// Cleanup UI when combat ends
pub fn cleanup_combat_ui_on_end(
    state: Res<CombatState>,
    mut commands: Commands,
    roots: Query<Entity, With<CombatUiRoot>>,
    p_bars: Query<Entity, With<PlayerHpBar>>,
    e_bars: Query<Entity, With<EnemyHpBar>>,
    lefts: Query<Entity, With<CombatUiLeftText>>,
    rights: Query<Entity, With<CombatUiRightText>>,
    logs: Query<Entity, With<CombatUiLogText>>,
    mut clog: ResMut<CombatLog>,
) {
    if state.is_changed() && state.active.is_none() {
        for e in roots.iter() { commands.entity(e).despawn(); }
        for e in p_bars.iter() { commands.entity(e).despawn(); }
        for e in e_bars.iter() { commands.entity(e).despawn(); }
        for e in lefts.iter() { commands.entity(e).despawn(); }
        for e in rights.iter() { commands.entity(e).despawn(); }
        for e in logs.iter() { commands.entity(e).despawn(); }
        // reset log
        clog.last_player = None;
        clog.last_enemy = None;
        clog.last_msg = None;
    }
}

// Show Game Over / Game Won overlay and handle restart
pub fn show_outcome_overlay(
    outcome: Res<GameOutcome>,
    mut commands: Commands,
    existing: Query<Entity, With<OutcomeUi>>,
    cam_q: Query<&Transform, With<Camera2d>>,
) {
    if outcome.is_changed() {
        // clear existing
        for e in existing.iter() { commands.entity(e).despawn(); }
        if !matches!(*outcome, GameOutcome::None) {
            // Full-screen tinted overlay sprite
            let color = match *outcome {
                GameOutcome::GameOver => Color::srgb(0.6, 0.0, 0.0).with_alpha(0.6),
                GameOutcome::GameWon => Color::srgb(0.0, 0.6, 0.0).with_alpha(0.6),
                GameOutcome::None => Color::BLACK
            };

            let cam_pos = cam_q.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::new(2000.0, 2000.0)), ..default() },
                // Keep overlay above other UI and sprites but within camera range
                Transform::from_translation(Vec3::new(cam_pos.x, cam_pos.y, 70.0)),
                OutcomeUi,
            ));

            // Centered text label for outcome
            let (label, color) = match *outcome {
                GameOutcome::GameOver => ("GAME OVER", Color::WHITE),
                GameOutcome::GameWon => ("YOU WIN", Color::WHITE),
                GameOutcome::None => ("", Color::WHITE),
            };
            commands.spawn((
                Text2d::new(label),
                TextFont { font_size: 64.0, ..Default::default() },
                TextColor(color),
                TextLayout { justify: Justify::Center, ..Default::default() },
                Transform::from_translation(Vec3::new(cam_pos.x, cam_pos.y, 71.0)),
                OutcomeUi,
            ));
        }
    }
}

pub fn handle_restart_input(
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CombatState>,
    mut outcome: ResMut<GameOutcome>,
    mut enemy_tracker: ResMut<EnemyTracker>,
    mut commands: Commands,
    enemies_q: Query<Entity, With<Enemy>>,
    mut player_q: Query<(&mut CombatStats, &mut Transform, &mut crate::characters::animation::AnimationController, &mut crate::characters::animation::AnimationState), With<Player>>,
    health_pips_q: Query<Entity, With<crate::characters::health::HealthPip>>,
    mut pip_tracker: ResMut<crate::characters::health::HealthPipTracker>,
) {
    // Allow restarting with R at any time
    // let on_outcome = !matches!(*outcome, GameOutcome::None);
    let restart_pressed = input.just_pressed(KeyCode::KeyR);
        // add back to restart_pressed if we want to restart on space or enter
        // || (on_outcome && (input.just_pressed(KeyCode::Space) || input.just_pressed(KeyCode::Enter)));

    if restart_pressed {
        // End any active combat session immediately
        state.active = None;

        // Despawn any existing enemies
        for e in enemies_q.iter() {
            commands.entity(e).despawn();
        }
        // Reset trackers so enemies will respawn
        enemy_tracker.spawned = false;
        enemy_tracker.alive = 0;

        // Restore player
        if let Ok((mut stats, mut tf, mut ctrl, mut astate)) = player_q.single_mut() {
            stats.hp = stats.max_hp;
            tf.translation.x = 0.0;
            tf.translation.y = 0.0;
            // Reset animation to a neutral state so we are no longer stuck in Death
            ctrl.current_animation = crate::characters::config::AnimationType::Walk;
            astate.is_moving = false;
        }

        // Clear health pips and reset tracker so they respawn
        for e in health_pips_q.iter() { commands.entity(e).despawn(); }
        pip_tracker.spawned = false;

        *outcome = GameOutcome::None;
    }
}

// Global Quit keybind: press Q to exit the app
pub fn handle_quit_input(
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyQ) {
        // Fallback quit: immediately terminate the process
        std::process::exit(0);
    }
}