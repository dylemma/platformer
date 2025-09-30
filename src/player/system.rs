use crate::player::{HorizontalControlParams, Player, PlayerControlParams, PlayerControlState};
use crate::util::{PlayerWallState, Side, SideMap, YSide};
use crate::{Platform, PlayerStatusText};
use bevy::input::ButtonInput;
use bevy::log::{debug, info};
use bevy::math::Vec2;
use bevy::prelude::{Entity, Gizmos, KeyCode, Query, Res, Text, Time, Transform, With};
use bevy_rapier2d::control::{KinematicCharacterController, KinematicCharacterControllerOutput};
use bevy_rapier2d::geometry::Collider;
use bevy_rapier2d::plugin::ReadRapierContext;
use std::f32;
use bevy::asset::Assets;

pub fn player_system(
    kb: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut PlayerControlState,
        &mut KinematicCharacterController,
        &KinematicCharacterControllerOutput,
        &Transform,
        &Collider,
    )>,
    control_params: Res<Assets<PlayerControlParams>>,
    mut status_text_query: Query<&mut Text, With<PlayerStatusText>>,
    obstacles: Query<(), With<Platform>>,
    time: Res<Time>,
    rapier_context: ReadRapierContext,
    mut gizmos: Gizmos,
) {
    let rapier_context = rapier_context.single();

    let mut status_text = status_text_query.single_mut();

    for (
        player_entity,
        player_component,
        mut player,
        mut controller,
        last_controller_out,
        player_transform,
        player_collider,
    ) in &mut player_query
    {
        if let Some(player_params) = control_params.get(player_component.0.id()) {

            // Check if the player wants to jump
            let wants_to_jump = {
                player.jump_requested.tick(kb.just_pressed(KeyCode::Space));
                player.jump_requested.was_set_within(player_params.jump_input_buffer)
            };

            // manage jump cooldown (more important when double-jump is enabled)
            player.jump_cooldown.tick();

            // sync Rapier controller state back to player
            player.grounded.tick(last_controller_out.grounded);

            // update timers related to wall-jumping
            player.wall_jump_force.tick();
            player.wall_jump_input_cooldown.tick();

            // if the player wall-jumped the last several frames,
            // stop them from trying to move back towards that wall
            let horizontal_input = {
                let desired = match (kb.pressed(KeyCode::KeyA) || kb.pressed(KeyCode::ArrowLeft), kb.pressed(KeyCode::KeyD) || kb.pressed(KeyCode::ArrowRight)) {
                    (true, false) => Some(Side::Left),
                    (false, true) => Some(Side::Right),
                    _ => None,
                };
                if !player.wall_jump_input_cooldown.is_ready() && desired == player.wall_jump_latest_side {
                    None
                } else {
                    desired
                }
            };
            let vertical_input = match (kb.pressed(KeyCode::KeyW), kb.pressed(KeyCode::KeyS)) {
                (true, false) => Some(YSide::Up),
                (false, true) => Some(YSide::Down),
                _ => None,
            };

            // if player ran into a platform, reset the portion of their velocity that caused that collision.
            // e.g. bonk your head when you jump into the ceiling, or stop when you run into a wall
            for collision in &last_controller_out.collisions {
                if let Ok(_) = obstacles.get(collision.entity) {
                    if let Some(hit) = collision.hit.details {
                        let normal = hit.normal1;

                        let prev_player_vel = player.own_velocity;
                        let arrested_velocity = -prev_player_vel.dot(normal) * normal;

                        debug!(
                            "player hit platform with normal {:?} and should adjust velocity by {:?}",
                            normal, arrested_velocity
                        );
                        player.own_velocity += arrested_velocity;

                        // TODO: if only a corner of the player actually clipped the wall/ceiling, push them around the corner
                    }
                }
            }

            // update wall sensors
            let wall_sensor_state = {
                let player_center = player_transform.translation.truncate();
                let player_half_extents = player_collider
                    .as_cuboid()
                    .unwrap_or_else(|| panic!("player collider isn't a cuboid"))
                    .half_extents();

                player.wall_sensors.update(
                    player_center,
                    player_half_extents,
                    player_params.wall_control_params.detection_length,
                    &rapier_context,
                    player_entity,
                );
                player
                    .wall_sensors
                    .draw(player_center, player_half_extents, &mut gizmos);

                SideMap {
                    left: player.wall_sensors.interpret(Side::Left),
                    right: player.wall_sensors.interpret(Side::Right),
                }
            };

            // refund jump ability when reaching the ground
            if player.grounded.is_set() {
                player.jumps_remaining = player_params.max_jumps;
                player.jumping = false;
                player.lost_jump_due_to_falling = false;

                if let Some(x_when_jumped) = player.x_when_jumped.take() {
                    let x_when_landed = player_transform.translation.x;
                    info!("Jumped from {:?} to {:?} (distance: {:?})!", x_when_jumped, x_when_landed, x_when_landed - x_when_jumped);
                }

            } else if !player.grounded.was_set_within(player_params.coyote_time) {
                // If player walks off a platform without jumping, then they lose a jump.
                // For a player with at most 1 jump, that just means they start falling normally.
                // We use "Coyote Time" per Looney Tunes logic, so this doesn't happen until
                // slightly after leaving the ground. The effect is a better feeling for the player,
                // since they don't need to be "frame perfect" with their jump input while trying
                // to wait until the last instant to jump.
                if !player.lost_jump_due_to_falling && !player.jumping {
                    player.jumps_remaining = player.jumps_remaining.saturating_sub(1);
                    player.lost_jump_due_to_falling = true;
                }
            }

            let player_wall_state = {
                let is_airborne = !player.grounded.is_set();
                let horizontal_momentum = match player.previous_total_velocity.x {
                    0.0 => None,
                    x => {
                        if x.is_sign_negative() {
                            Some(Side::Left)
                        } else {
                            Some(Side::Right)
                        }
                    }
                };
                player.wall_control_state.tick(
                    &wall_sensor_state,
                    is_airborne,
                    &player_params.wall_control_params,
                    horizontal_input,
                    horizontal_momentum,
                    vertical_input,
                )
            };

            // update player's "run/float" based on horizontal inputs
            player.own_velocity.x = {
                let filtered_horizontal_input = if player_wall_state.is_some() {
                    None
                } else {
                    horizontal_input
                };
                compute_next_horizontal_velocity(
                    player.own_velocity.x,
                    filtered_horizontal_input,
                    if player.grounded.is_set() {
                        player_params.run
                    } else {
                        player_params.float
                    },
                )
            };

            // apply gravity (when not already on the ground or stuck to a wall)
            if player.grounded.is_set() {
                player.own_velocity.y = 0.0;
            } else if let Some(wall_state) = player_wall_state {
                let vy = player.own_velocity.y;
                match wall_state {
                    PlayerWallState::Grabbed(_) => {
                        // apply gravity to arrest upward momentum, but don't let the player slide down
                        player.own_velocity.y = (vy + player_params.gravity).max(0.0);
                    }
                    PlayerWallState::Sliding(_) => {
                        // apply normal gravity to arrest upward momentum,
                        // but downward force should be gentle
                        if vy >= -player_params.gravity {
                            player.own_velocity.y += player_params.gravity;
                        } else if vy > 0.0 {
                            player.own_velocity.y = 0.0
                        } else {
                            player.own_velocity.y = (vy - player_params.wall_control_params.slide_acceleration)
                                .max(-player_params.wall_control_params.slide_max_speed);
                        }
                    }
                    PlayerWallState::Climbing(_) => {
                        // let the player climb up the ledge
                        let climb_max = player_params.wall_control_params.climb_max_speed;
                        let climb_accel = player_params.wall_control_params.climb_acceleration;
                        if vy < climb_max {
                            // if they weren't moving upwards (fast), accelerate them upward
                            player.own_velocity.y = (vy + climb_accel).min(climb_max).max(0.0);
                        } else {
                            // if they are already moving upwards quickly, let gravity apply
                            // until they reach the normal climbing speed
                            player.own_velocity.y = (vy + player_params.gravity).min(climb_max);
                        }
                    }
                }
            } else {
                // apply normal gravity
                player.own_velocity.y += player_params.gravity;
                if player.own_velocity.y <= 0.0 {
                    if let Some(y_when_jumped) = player.y_when_jumped.take() {
                        let apex = player_transform.translation.y;
                        info!("Jumped apex {:?} to {:?} (distance: {:?})!", y_when_jumped, apex, apex - y_when_jumped);
                    }
                }
            }

            // jump
            if wants_to_jump && player.jump_cooldown.is_ready() {
                if let Some(wall_state) = player_wall_state.as_ref() {
                    // wall jump
                    debug!("wall jumping from {:?} wall!", wall_state.side());
                    // although effectively a vector, the X and Y components will be split;
                    // the Y trajectory will be applied normally, but the X trajectory
                    // will be applied as an "external force" so the player's run/float
                    // control logic doesn't completely overwrite the force too soon
                    let jump_vx = player_params.run.max_speed * f32::consts::FRAC_1_SQRT_2 * -wall_state.side();
                    let jump_vy = player_params.jump_speed * f32::consts::FRAC_1_SQRT_2;
                    player.wall_jump_force.reset(Vec2::new(jump_vx, 0.0));
                    player.own_velocity.y = jump_vy;
                    player.x_when_jumped = Some(player_transform.translation.x);
                    player.y_when_jumped = Some(player_transform.translation.y);
                    player.jumping = true;
                    player.jump_cooldown.reset(player_params.jump_cooldown);
                    player
                        .wall_jump_input_cooldown
                        .reset(player_params.wall_jump_input_cooldown);
                    player.wall_jump_latest_side = Some(wall_state.side());
                    player.wall_control_state.release();
                } else if player.jumps_remaining > 0 {
                    // normal jump
                    debug!("jumping with coyote time {:?}", player.grounded);
                    player.own_velocity.y = player_params.jump_speed;
                    player.jumps_remaining -= 1;
                    player.x_when_jumped = Some(player_transform.translation.x);
                    player.y_when_jumped = Some(player_transform.translation.y);
                    player.jumping = true;
                    player.jump_cooldown.reset(player_params.jump_cooldown);
                }
            }

            // finish velocity computation
            let wall_jump_force = player.wall_jump_force.eval(&player_params.wall_jump_force_decay);
            let player_velocity_per_sec = player.own_velocity + wall_jump_force;
            player.previous_total_velocity = player_velocity_per_sec;

            // debug text for velocity
            status_text.0 = format!(
                "vx: {}\nvy: {}\ngrounded: {}\njumps: {}\nwalljump: {:?}\nwall_state: {:?}",
                player_velocity_per_sec.x,
                player_velocity_per_sec.y,
                player.grounded.is_set(),
                player.jumps_remaining,
                wall_jump_force,
                player_wall_state,
            );

            // send computed translation to controller for resolution in the physics world
            controller.translation = Some(player_velocity_per_sec * time.delta_secs());
        } else {
            info!("player params not loaded yet");
        }
    }
}

/// Solve for a player's new horizontal velocity by accelerating or decelerating
/// their current velocity towards their desired velocity
fn compute_next_horizontal_velocity(
    current_vel: f32,
    input_direction: Option<Side>,
    HorizontalControlParams {
        max_speed,
        acceleration,
        deceleration,
    }: HorizontalControlParams,
) -> f32 {
    let target_vel = match input_direction {
        None => 0.0,
        Some(Side::Left) => -max_speed,
        Some(Side::Right) => max_speed,
    };

    let accel_base = if current_vel == 0.0 {
        // anything is faster than 0, regardless of direction
        acceleration
    } else if input_direction.is_none() {
        // if the goal is to stop, that's always deceleration
        deceleration
    } else if target_vel.signum() != current_vel.signum() {
        // if the goal is in the opposite direction, decelerate to 0 first
        deceleration
    } else if max_speed > current_vel.abs() {
        // previous conditions ensure `target_vel` and `current_vel` have the same sign,
        // so this means we need to speed up (in whichever direction) to reach max speed
        acceleration
    } else {
        // player is already moving faster than `max_speed`, which could happen if they
        // had previously accelerated in a different mode (e.g. running vs floating).
        // Instead of decelerating down to the new max, the player can keep their "momentum",
        // and we will neither accelerate nor decelerate
        0.0
    };

    // What's the overall change in velocity the player wants to achieve?
    let goal_delta = target_vel - current_vel;

    // If the acceleration is more than enough to reach the goal this frame,
    // do so and skip some math
    if goal_delta.abs() < accel_base {
        target_vel
    } else {
        // Apply acceleration in the direction of the goal
        let accel_amount = accel_base * goal_delta.signum();
        current_vel + accel_amount
    }
}