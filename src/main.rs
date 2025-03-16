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
		// .add_systems(FixedUpdate, read_result_system.after(player_system))
		//
		// rapier physics
		//
		.insert_resource(TimestepMode::Fixed {
			dt: 1. / 60.,
			substeps: 1,
		})
		.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(10.0).in_fixed_schedule())
		.add_plugins(RapierDebugRenderPlugin::default())
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

#[derive(Component)]
#[require(PlayerControlState)]
struct Player {
	max_run_speed: f32,
	run_acceleration: f32,
	run_deceleration: f32,
	jump_speed: f32,
	gravity: f32,
	coyote_time_frames: u32,
	jump_buffer_frames: u32,
	max_jumps: u8,
	jump_cooldown_frames: u8,
	wall_release_jump_deadline_frames: u8,
	// air_max_speed: f32,
	// air_acceleration: f32,
	// air_deceleration: f32,
	// current_air_speed: f32,
}

#[derive(Component, Default)]
struct PlayerControlState {
	grounded: bool,
	jumping: bool,
	lost_jump_due_to_falling: bool,
	current_run_speed: f32,
	current_fall_speed: f32, // TODO: naming is kinda backwards because positive means "going up"
	frames_since_grounded: u32,
	frames_since_jump_input: u32,
	jumps_remaining: u8,
	frames_since_jumped: u8,
	wall_grab_sensors: WallGrabSensors,
	wall_grab_state: WallGrabState,
}

#[derive(Component)]
struct PlayerStatusText;

fn setup_player(mut commands: Commands) {
	commands.spawn((
		Player {
			max_run_speed: 90.0,
			// accelerate to max speed in a quarter second (15 frames)
			run_acceleration: 6.0,
			// decelerate from max speed to 0 in 1/10 second (6 frames)
			run_deceleration: 15.0,
			jump_speed: 150.0,
			gravity: -10.0,
			coyote_time_frames: 6,
			jump_buffer_frames: 6,
			max_jumps: 1, // can be set to 2, to allow double-jump
			jump_cooldown_frames: 8,
			// air_max_speed: 25.0,
			// air_acceleration: 5.0,
			// air_deceleration:
			wall_release_jump_deadline_frames: 10,
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

	let jump_requested = kb.just_pressed(KeyCode::Space);

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

		// update player's "run" based on horizontal inputs
		player.current_run_speed = compute_run_velocity(
			/* current */ player.current_run_speed,
			/* desired */ input_axis.x * player_params.max_run_speed,
			/* accel  */ player_params.run_acceleration,
			/* decel  */ player_params.run_deceleration,
		);

		// sync Rapier controller state back to player
		player.grounded = last_controller_out.grounded;

		// refund jump ability when reaching the ground
		if player.grounded {
			player.jumps_remaining = player_params.max_jumps;
			player.jumping = false;
			player.lost_jump_due_to_falling = false;
		}

		// coyote timer
		if player.grounded {
			player.frames_since_grounded = 0;
		} else {
			player.frames_since_grounded = player.frames_since_grounded.saturating_add(1);
		}

		// if player walks off a platform without jumping, then they lose a jump
		if player.frames_since_grounded > player_params.coyote_time_frames {
			if !player.lost_jump_due_to_falling && !player.jumping {
				player.jumps_remaining = player.jumps_remaining.saturating_sub(1);
				player.lost_jump_due_to_falling = true;
			}
		}

		// jump-buffering
		if jump_requested {
			player.frames_since_jump_input = 0;
		} else {
			player.frames_since_jump_input = player.frames_since_jump_input.saturating_add(1);
		}

		// manage jump cooldown (more important when double-jump is enabled)
		player.frames_since_jumped = player.frames_since_jumped.saturating_add(1);

		// detect wall-grab
		let currently_grabbed = if player.grounded {
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
		if player.grounded {
			player.current_fall_speed = 0.0;
		} else if currently_grabbed.is_some() && player.current_fall_speed <= 0.0 {
			// note the `<=` which seems redundant, because "why set it to 0 when it's already 0",
			// but it's important to prevent the `falling` case from triggering every other frame
			player.current_fall_speed = 0.0;
		} else {
			player.current_fall_speed += player_params.gravity;
		}

		// normal jump
		let wants_to_jump = player.frames_since_jump_input <= player_params.jump_buffer_frames;
		let can_jump = player.jumps_remaining > 0 && player.frames_since_jumped > player_params.jump_cooldown_frames;
		if wants_to_jump && can_jump {
			debug!("jumping with coyote time {}", player.frames_since_grounded);
			player.current_fall_speed = player_params.jump_speed;
			player.jumps_remaining -= 1;
			player.jumping = true;
			player.frames_since_jumped = 0;
		}

		// wall jump
		if let Some(side) = player
			.wall_grab_state
			.latest_grabbed_within(player_params.wall_release_jump_deadline_frames)
		{
			let can_wall_jump = player.frames_since_jumped > player_params.jump_cooldown_frames;
			if wants_to_jump && can_wall_jump {
				debug!("wall jumping from {:?} wall!", side);
				player.current_run_speed = player_params.max_run_speed * f32::consts::FRAC_1_SQRT_2 * -side;
				player.current_fall_speed = player_params.jump_speed;
				player.jumping = true;
				player.frames_since_jumped = 0;
			}
		}

		// finish velocity computation
		let player_velocity_per_sec = Vec2::new(player.current_run_speed, player.current_fall_speed);

		// debug text for velocity
		status_text.0 = format!(
			"vx: {}\nvy: {}\ngrounded: {}\njumps: {}",
			player_velocity_per_sec.x, player_velocity_per_sec.y, player.grounded, player.jumps_remaining,
		);

		// send computed translation to controller for resolution in the physics world
		controller.translation = Some(player_velocity_per_sec * time.delta_secs());
	}
}

/// Solve for a player's new horizontal velocity, by accelerating or decelerating
/// their current velocity towards their desired velocity
fn compute_run_velocity(current_vel: f32, desired_vel: f32, acceleration: f32, deceleration: f32) -> f32 {
	// The player will have a separate "speed up" and "slow down" rate;
	// Pick the appropriate one based on the difference between current
	// and desired velocities.
	let accel_base = if current_vel == 0.0 {
		// anything is faster than 0, regardless of direction
		acceleration
	} else if desired_vel == 0.0 {
		// if the goal is to stop, that's always deceleration
		deceleration
	} else if desired_vel.signum() != current_vel.signum() {
		// if the goal is in the opposite direction, decelerate to 0 first
		deceleration
	} else if desired_vel.abs() < current_vel.abs() {
		// same direction but goal speed is slower; decelerate
		deceleration
	} else {
		// gotta go fast!
		acceleration
	};

	// What's the overall change in velocity the player wants to achieve?
	let goal_delta = desired_vel - current_vel;

	// If the acceleration is more than enough to reach the goal this frame,
	// do so and skip some math
	if goal_delta.abs() < accel_base {
		desired_vel
	} else {
		// Apply acceleration in the direction of the goal
		let accel_amount = accel_base * goal_delta.signum();
		current_vel + accel_amount
	}
}

// TODO: probably just delete this; it was an idea to provide a structure for things like
//       jump cooldowns and coyote timers, but it's pretty half-baked
// #[derive(Default)]
// struct TimedFlag {
// 	value: bool,
// 	time_at_current_value: u32,
// }
// impl TimedFlag {
// 	fn new(value: bool) -> Self {
// 		Self {
// 			value,
// 			time_at_current_value: 0,
// 		}
// 	}
// 	fn tick(&mut self) {
// 		self.time_at_current_value += 1;
// 	}
// 	fn set(&mut self, value: bool) {
// 		if value != self.value {
// 			self.time_at_current_value = 0;
// 		}
// 		self.value = value;
// 	}
// 	fn has_been_true_at_most(&self, time: u32) -> bool {
// 		self.value && self.time_at_current_value <= time
// 	}
// }
