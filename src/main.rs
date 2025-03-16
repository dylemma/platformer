mod util;

use crate::util::*;
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy_rapier2d::prelude::*;
use std::f32;

fn main() {
	App::new()
		// baseline bevy stuff
		.add_plugins(DefaultPlugins)
		.insert_resource(Time::<Fixed>::from_hz(60.))
		//
		// platformer learning zone
		.add_systems(Startup, setup_camera)
		.add_systems(Startup, setup_player)
		.add_systems(Startup, setup_platforms)
		.add_systems(FixedUpdate, player_system)
		//
		// rapier physics
		//
		.insert_resource(TimestepMode::Fixed {
			dt: 1. / 60.,
			substeps: 1,
		})
		.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(10.0).in_fixed_schedule())
		// .add_plugins(RapierDebugRenderPlugin::default())
		.run();
}

fn setup_camera(mut commands: Commands) {
	commands.spawn((
		Camera2d,
		Transform::from_xyz(50.0, 50.0, 1.0),
		OrthographicProjection {
			scaling_mode: ScalingMode::AutoMin {
				min_width: 100.,
				min_height: 100.,
			},
			..OrthographicProjection::default_2d()
		},
	));
}

#[derive(Component)]
struct Platform;

struct WallArgs {
	color: Color,
	pos: Vec2,
	size: Vec2,
}
impl WallArgs {
	fn spawn(self, commands: &mut Commands) {
		let WallArgs { color, pos, size } = self;
		commands.spawn((
			Platform,
			RigidBody::Fixed,
			Sprite::from_color(color, size),
			Collider::cuboid(size.x * 0.5, size.y * 0.5),
			Transform::from_xyz(pos.x, pos.y, 0.0),
		));
	}
}

fn setup_platforms(mut commands: Commands, asset_server: Res<AssetServer>) {
	// background
	commands.spawn((
		Sprite::from_color(Color::srgba(0., 0.5, 0.75, 0.2), Vec2::new(100., 100.)),
		Transform::from_xyz(50., 50., 0.),
	));

	// floor
	WallArgs {
		color: Color::srgb(0.15, 0.8, 0.25),
		pos: Vec2::new(50., 3.),
		size: Vec2::new(98.0, 4.0),
	}
	.spawn(&mut commands);

	// platform 1
	WallArgs {
		color: Color::srgb(0.15, 0.8, 0.25),
		pos: Vec2::new(75.0, 18.0),
		size: Vec2::new(20.0, 4.0),
	}
	.spawn(&mut commands);

	// platform 2
	WallArgs {
		color: Color::srgb(0.15, 0.8, 0.25),
		pos: Vec2::new(50.0, 35.0),
		size: Vec2::new(20.0, 2.0),
	}
	.spawn(&mut commands);

	// west wall
	WallArgs {
		color: Color::srgb(0.15, 0.5, 0.15),
		pos: Vec2::new(3., 50.),
		size: Vec2::new(4.0, 98.0),
	}
	.spawn(&mut commands);

	// east wall
	WallArgs {
		color: Color::srgb(0.45, 0.5, 0.15),
		pos: Vec2::new(97., 50.),
		size: Vec2::new(4.0, 98.0),
	}
	.spawn(&mut commands);

	// ceiling
	WallArgs {
		color: Color::srgb(0.45, 0.8, 0.25),
		pos: Vec2::new(50., 97.),
		size: Vec2::new(98.0, 4.0),
	}
	.spawn(&mut commands);

	// a ball to bounce around
	// TODO: why does the kinematic character controller seem to get "stuck" on the ball
	//       when attempting to start pushing it from stationary?
	commands.spawn((
		RigidBody::Dynamic,
		Sprite {
			color: Color::srgb(0., 0.75, 0.0),
			custom_size: Some(Vec2::new(4., 4.)),
			..Sprite::from_image(asset_server.load("circle_32x32.png"))
		},
		Transform::from_xyz(50., 50., 0.),
		Collider::ball(2.),
		Restitution::coefficient(0.75),
		GravityScale(1.0),
		Velocity::linear(Vec2::new(200.0, 200.0)),
		Ccd::enabled(),
	));
}

#[derive(Copy, Clone, Debug)]
struct HorizontalControlParams {
	max_speed: f32,
	acceleration: f32,
	deceleration: f32,
}

#[derive(Component)]
#[require(PlayerControlState)]
struct Player {
	run: HorizontalControlParams,
	float: HorizontalControlParams,
	jump_speed: f32,
	gravity: f32,
	coyote_time: FrameCount,
	jump_input_buffer: FrameCount,
	max_jumps: u8,
	jump_cooldown: FrameCount,
	wall_release_jump_deadline: FrameCount,
}

#[derive(Component, Default)]
struct PlayerControlState {
	grounded: CapacitiveFlag,
	jumping: bool,
	lost_jump_due_to_falling: bool,
	current_run_speed: f32,
	current_fall_speed: f32, // TODO: naming is kinda backwards because positive means "going up"
	jump_requested: CapacitiveFlag,
	jumps_remaining: u8,
	jump_cooldown: Cooldown,
	wall_grab_sensors: WallGrabSensors,
	wall_grab_state: WallGrabState,
}

#[derive(Component)]
struct PlayerStatusText;

fn setup_player(mut commands: Commands) {
	commands.spawn((
		Player {
			run: HorizontalControlParams {
				max_speed: 90.0,
				acceleration: 6.0,  // 15 frames (0.25s) to accelerate to max
				deceleration: 15.0, // 6 frames (0.1s) to stop from max
			},
			float: HorizontalControlParams {
				max_speed: 60.0,
				acceleration: 3.0,  // 20 frames (0.33s) to accelerate to max
				deceleration: 15.0, // 4 frames (0.066s) to stop from max (or 6 frames from max run speed)
			},
			jump_speed: 150.0,
			gravity: -10.0,
			coyote_time: FrameCount(6),
			jump_input_buffer: FrameCount(4),
			max_jumps: 1, // can be set to 2, to allow double-jump
			jump_cooldown: FrameCount(8),
			wall_release_jump_deadline: FrameCount(10),
		},
		Friction {
			coefficient: 0.0,
			combine_rule: CoefficientCombineRule::Multiply,
		},
		Sprite::from_color(Color::srgb(1., 0.5, 0.), Vec2::new(3.0, 5.0)),
		Collider::cuboid(1.5, 2.5),
		Transform::from_xyz(25., 25., 0.),
		RigidBody::KinematicPositionBased,
		KinematicCharacterController {
			filter_flags: QueryFilterFlags::EXCLUDE_DYNAMIC,
			..default()
		},
		KinematicCharacterControllerOutput::default(),
	));

	// Debug text for player state
	commands.spawn((
		PlayerStatusText,
		Text::new("hello world"),
		TextLayout::new_with_justify(JustifyText::Right),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(10.0),
			right: Val::Px(10.0),
			..default()
		},
	));
}

fn player_system(
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
	mut status_text_query: Query<&mut Text, With<PlayerStatusText>>,
	obstacles: Query<(), With<Platform>>,
	time: Res<Time>,
	rapier_context: ReadRapierContext,
	mut gizmos: Gizmos,
) {
	let rapier_context = rapier_context.single();

	let mut status_text = status_text_query.single_mut();

	// TODO: maybe just replace this? Seems like we won't need a Vec2
	let input_axis = {
		let mut v = Vec2::new(0., 0.);
		if kb.pressed(KeyCode::KeyA) {
			v.x -= 1.0;
		}
		if kb.pressed(KeyCode::KeyD) {
			v.x += 1.0;
		}
		if kb.pressed(KeyCode::KeyW) {
			v.y += 1.0;
		}
		if kb.pressed(KeyCode::KeyS) {
			v.y -= 1.0;
		}
		v
	};
	let horizontal_input = match (kb.pressed(KeyCode::KeyA), kb.pressed(KeyCode::KeyD)) {
		(true, false) => Some(Side::Left),
		(false, true) => Some(Side::Right),
		_ => None,
	};

	for (
		player_entity,
		player_params,
		mut player,
		mut controller,
		last_controller_out,
		player_transform,
		player_collider,
	) in &mut player_query
	{
		// Check if the player wants to jump
		let wants_to_jump = {
			player.jump_requested.tick(kb.just_pressed(KeyCode::Space));
			player.jump_requested.was_set_within(player_params.jump_input_buffer)
		};

		// manage jump cooldown (more important when double-jump is enabled)
		player.jump_cooldown.tick();

		// sync Rapier controller state back to player
		player.grounded.tick(last_controller_out.grounded);

		// if player ran into a platform, reset the portion of their velocity that caused that collision.
		// e.g. bonk your head when you jump into the ceiling, or stop when you run into a wall
		for collision in &last_controller_out.collisions {
			if let Ok(_) = obstacles.get(collision.entity) {
				if let Some(hit) = collision.hit.details {
					let normal = hit.normal1;

					let prev_player_vel = Vec2::new(player.current_run_speed, player.current_fall_speed);
					let arrested_velocity = -prev_player_vel.dot(normal) * normal;

					debug!(
						"player hit platform with normal {:?} and should adjust velocity by {:?}",
						normal, arrested_velocity
					);
					player.current_run_speed += arrested_velocity.x;
					player.current_fall_speed += arrested_velocity.y;

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

			player
				.wall_grab_sensors
				.update(player_center, player_half_extents, &rapier_context, player_entity);
			player
				.wall_grab_sensors
				.draw(player_center, player_half_extents, &mut gizmos);

			SideMap {
				left: player.wall_grab_sensors.interpret(Side::Left),
				right: player.wall_grab_sensors.interpret(Side::Right),
			}
		};

		// update player's "run/float" based on horizontal inputs
		player.current_run_speed = compute_player_self_velocity(
			player.current_run_speed,
			horizontal_input,
			if player.grounded.is_set() {
				player_params.run
			} else {
				player_params.float
			},
		);

		// refund jump ability when reaching the ground
		if player.grounded.is_set() {
			player.jumps_remaining = player_params.max_jumps;
			player.jumping = false;
			player.lost_jump_due_to_falling = false;
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

		// detect wall-grab
		let currently_grabbed = if player.grounded.is_set() {
			None
		} else {
			// the player is grabbing a wall if their input is pointing in a nonzero horizontal direction,
			// and the wall sensor thinks there's a wall or ledge in that direction
			(match input_axis.x {
				0.0 => None,
				x if x.is_sign_negative() => Some(Side::Left),
				_ => Some(Side::Right),
			})
			.and_then(|side| {
				match wall_sensor_state[side] {
					WallInterpretation::Wall | WallInterpretation::Ledge => {
						// could wall jump off this!
						if player.current_fall_speed < 0.0 {}
						Some(side)
					}
					_ => None,
				}
			})
		};
		player.wall_grab_state.tick(currently_grabbed);

		// apply gravity (when not already on the ground or stuck to a wall)
		if player.grounded.is_set() {
			player.current_fall_speed = 0.0;
		} else if currently_grabbed.is_some() && player.current_fall_speed <= 0.0 {
			// note the `<=` which seems redundant, because "why set it to 0 when it's already 0",
			// but it's important to prevent the `falling` case from triggering every other frame
			player.current_fall_speed = 0.0;
		} else {
			player.current_fall_speed += player_params.gravity;
		}

		// jump
		if wants_to_jump && player.jump_cooldown.is_ready() {
			if let Some(side) = player
				.wall_grab_state
				.latest_grabbed_within(player_params.wall_release_jump_deadline)
			{
				// wall jump
				debug!("wall jumping from {:?} wall!", side);
				player.current_run_speed = player_params.run.max_speed * f32::consts::FRAC_1_SQRT_2 * -side;
				player.current_fall_speed = player_params.jump_speed;
				player.jumping = true;
				player.jump_cooldown.reset(player_params.jump_cooldown);
			} else if player.jumps_remaining > 0 {
				// normal jump
				debug!("jumping with coyote time {:?}", player.grounded);
				player.current_fall_speed = player_params.jump_speed;
				player.jumps_remaining -= 1;
				player.jumping = true;
				player.jump_cooldown.reset(player_params.jump_cooldown);
			}
		}

		// finish velocity computation
		let player_velocity_per_sec = Vec2::new(player.current_run_speed, player.current_fall_speed);

		// debug text for velocity
		status_text.0 = format!(
			"vx: {}\nvy: {}\ngrounded: {}\njumps: {}",
			player_velocity_per_sec.x,
			player_velocity_per_sec.y,
			player.grounded.is_set(),
			player.jumps_remaining,
		);

		// send computed translation to controller for resolution in the physics world
		controller.translation = Some(player_velocity_per_sec * time.delta_secs());
	}
}

/// Solve for a player's new horizontal velocity, by accelerating or decelerating
/// their current velocity towards their desired velocity
fn compute_player_self_velocity(
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
