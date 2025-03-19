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
		pos: Vec2::new(50.0, 30.0),
		size: Vec2::new(20.0, 2.0),
	}
	.spawn(&mut commands);

	WallArgs {
		color: Color::srgb(0.15, 0.8, 0.25),
		pos: Vec2::new(35.0, 50.0),
		size: Vec2::new(2.0, 20.0),
	}
	.spawn(&mut commands);

	WallArgs {
		color: Color::srgb(0.15, 0.8, 0.25),
		pos: Vec2::new(50.0, 58.0),
		size: Vec2::new(2.0, 20.0),
	}
	.spawn(&mut commands);

	WallArgs {
		color: Color::srgb(0.15, 0.8, 0.25),
		pos: Vec2::new(28.0, 68.0),
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
	wall_jump_force_decay: ForceDecayCurve,
	wall_jump_input_cooldown: FrameCount,
	wall_control_params: PlayerWallControlParams,
}

#[derive(Component, Default)]
struct PlayerControlState {
	/// tracks whether the player is on the ground, or how recently they were on the ground
	grounded: CapacitiveFlag,

	/// tracks whether the player is airborne as the result of a jump (as opposed to falling)
	jumping: bool,

	/// flag used to avoid decrementing `jumps_remaining` every frame while in midair
	lost_jump_due_to_falling: bool,

	/// Velocity derived from the directional inputs, and gravity.
	/// Does not exclude "external forces" affecting the X direction, like
	/// wall jumping or moving platforms
	own_velocity: Vec2,

	/// Input buffer for jumping
	jump_requested: CapacitiveFlag,

	/// Resource counter for the player's jumps. Decrements when jumping from the ground or falling
	/// from a platform. Resets when landing on the ground
	jumps_remaining: u8,

	/// cooldown timer for jumping
	jump_cooldown: Cooldown,

	/// a sensor object used to detect walls, ledges, and steps adjacent to the player
	wall_sensors: WallSensors,

	/// a decaying force that is added when wall-jumping
	wall_jump_force: TemporaryForce,

	/// amount of time after wall jumping, where attempting to move back towards the wall will be ignored
	wall_jump_input_cooldown: Cooldown,

	/// tracks the side that the wall was on, when jumping from it
	wall_jump_latest_side: Option<Side>,

	/// state that becomes active when the player comes in contact with a wall while airborne
	wall_control_state: PlayerWallControlState,

	/// remembers the total computed velocity (per-second) from the previous update
	previous_total_velocity: Vec2,
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
			jump_speed: 120.0,
			gravity: -8.0,
			coyote_time: FrameCount(6),
			jump_input_buffer: FrameCount(4),
			max_jumps: 1, // can be set to 2, to allow double-jump
			jump_cooldown: FrameCount(8),
			wall_jump_force_decay: ForceDecayCurve {
				easing: EaseFunction::Linear,
				duration: FrameCount(20),
			},
			wall_jump_input_cooldown: FrameCount(5),
			wall_control_params: PlayerWallControlParams {
				push_away_duration: FrameCount(20),
				slide_max_speed: 20.0,
				slide_acceleration: 0.5,
				climb_max_speed: 10.0,
				climb_acceleration: 2.0,
			},
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

		// update timers related to wall-jumping
		player.wall_jump_force.tick();
		player.wall_jump_input_cooldown.tick();

		// if the player wall-jumped the last several frames,
		// stop them from trying to move back towards that wall
		let horizontal_input = {
			let desired = match (kb.pressed(KeyCode::KeyA), kb.pressed(KeyCode::KeyD)) {
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

			player
				.wall_sensors
				.update(player_center, player_half_extents, &rapier_context, player_entity);
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
	}
}

/// Solve for a player's new horizontal velocity, by accelerating or decelerating
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

struct ForceDecayCurve {
	easing: EaseFunction,
	duration: FrameCount,
}

#[derive(Default)]
struct TemporaryForce {
	age: FrameCount,
	max: Vec2,
}

impl TemporaryForce {
	fn eval(&self, curve: &ForceDecayCurve) -> Vec2 {
		if self.age > curve.duration || curve.duration.0 == 0 {
			// force expired, or the curve is undefined with 0 duration
			Vec2::ZERO
		} else {
			// calculate the ratio of age/duration from 0 to 1,
			// then use that as input the easing curve to get the
			// current fraction of the `max` force
			let t = self.age.0 as f32 / curve.duration.0 as f32;
			let magnitude = EasingCurve::new(1.0, 0.0, curve.easing).sample_unchecked(t);
			self.max * magnitude
		}
	}
	fn tick(&mut self) {
		self.age.increment();
	}
	fn reset(&mut self, max: Vec2) {
		self.max = max;
		self.age = FrameCount(0);
	}
}
