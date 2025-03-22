use crate::util::{FrameCount, Side, SideMap, YSide};
use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy_rapier2d::pipeline::{QueryFilter, QueryFilterFlags};
use bevy_rapier2d::plugin::RapierContext;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PlayerWallState {
	Grabbed(Side),
	Sliding(Side),
	Climbing(Side),
}

impl PlayerWallState {
	pub fn side(&self) -> Side {
		match *self {
			PlayerWallState::Grabbed(side) => side,
			PlayerWallState::Sliding(side) => side,
			PlayerWallState::Climbing(side) => side,
		}
	}
}

struct PlayerWallControlStateInner {
	/// The direction from the player to the wall
	side: Side,

	/// Duration the player has been pressing the horizontal directional input *away* from the wall
	push_away_timer: FrameCount,

	/// Remembers the type of wall (i.e. Wall vs Ledge) being interacted with
	wall_type: WallSensorResult,
}

#[derive(Default)]
pub struct PlayerWallControlState {
	wall_state: Option<PlayerWallControlStateInner>,
}

impl PlayerWallControlState {
	/// Force the player to release the wall, if they were currently interacting with one.
	/// (expected usage is with wall-jumps, which are not handled by this struct)
	pub fn release(&mut self) {
		self.wall_state = None;
	}

	/// Advance the control state by one frame, taking into consideration the player's
	/// directional inputs and proximity to walls, and determining how (if at all) the
	/// player is interacting with a wall.
	pub fn tick(
		&mut self,
		wall_sensor_results: &SideMap<WallSensorResult>,
		player_is_airborne: bool,
		control_params: &PlayerWallControlParams,
		horizontal_input: Option<Side>,
		horizontal_momentum: Option<Side>,
		vertical_input: Option<YSide>,
	) -> Option<PlayerWallState> {
		// Possibly enter the wall state:
		//   If player gets in contact with a wall while facing it, or gets thrown into
		//   it regardless of the direction they are facing, they should "attach" to the wall.
		if self.wall_state.is_none() && player_is_airborne {
			if let Some(player_side) = horizontal_momentum.or(horizontal_input) {
				match wall_sensor_results[player_side] {
					t @ (WallSensorResult::Wall | WallSensorResult::Ledge) => {
						info!("started interacting with wall on {:?}", player_side);
						// enter the wall state
						self.wall_state = Some(PlayerWallControlStateInner {
							side: player_side,
							push_away_timer: FrameCount(0),
							wall_type: t,
						});
					}
					_ => (),
				}
			}
		}

		// Possibly exit the wall state:
		//   If the player tries to push away from the wall for long enough,
		//   they'll "release" the wall.
		if let Some(wall_state) = &mut self.wall_state {
			let is_pushing_away = horizontal_input.map_or(false, |pushing_side| pushing_side != wall_state.side);

			if is_pushing_away {
				wall_state.push_away_timer.increment();
				if wall_state.push_away_timer >= control_params.push_away_duration {
					// they pushed for long enough; release the wall
					info!(
						"Released wall due to pressing away from it for {:?}",
						control_params.push_away_duration
					);
					self.wall_state = None;
				}
			} else {
				// player stopped pushing away
				wall_state.push_away_timer.reset()
			}
		}

		// Possibly exit the wall state:
		//   If the player pushes the Down button, they should let go of the wall
		if vertical_input == Some(YSide::Down) {
			info!("Released wall due to pressing Down");
			self.wall_state = None;
		}

		// Possibly exit the wall state:
		//   If the sensor no longer detects a wall
		if let Some(wall_state) = self.wall_state.as_ref() {
			match wall_sensor_results[wall_state.side] {
				WallSensorResult::Step | WallSensorResult::NotAWall => {
					self.wall_state = None;
				}
				_ => (),
			}
		}

		// Possibly exit the wall state:
		//   If the player is grounded
		if !player_is_airborne {
			self.wall_state = None;
		}

		// Interpret the state and the player's directional inputs
		// to determine what the character is actually doing
		self.wall_state.as_ref().map(|wall_state| {
			let is_ledge = match wall_state.wall_type {
				WallSensorResult::Ledge => true,
				_ => false,
			};

			if is_ledge && (vertical_input == Some(YSide::Up) || horizontal_input == Some(wall_state.side)) {
				// allow the player to climb up a ledge by holding either Up or towards the ledge
				PlayerWallState::Climbing(wall_state.side)
			} else if horizontal_input == Some(wall_state.side) {
				// on a normal wall, pressing towards the wall counts as grabbing it
				PlayerWallState::Grabbed(wall_state.side)
			} else {
				// pressing away from the wall, or in no direction at all, should result
				// in the player slowly sliding down the wall
				PlayerWallState::Sliding(wall_state.side)
			}
		})
	}
}

pub struct PlayerWallControlParams {
	/// Duration that player needs to hold the directional input away from the wall
	/// before they actually let go and start falling
	pub push_away_duration: FrameCount,
	pub slide_max_speed: f32,
	pub slide_acceleration: f32,
	pub climb_max_speed: f32,
	pub climb_acceleration: f32,

	/// Length of ray-casts used to detect walls adjacent to the player
	pub detection_length: f32,
}

/// Describes a sensor that exists at the sides of a player's collider,
/// projecting rays to each side to detect walls in a physics world.
#[derive(Default, Debug)]
pub struct WallSensor {
	/// Ratio value between 0.0 and 1.0 representing how far from the bottom of the
	/// player's collider this sensor exists
	local_offset: f32,

	/// Tracks whether the ray-casts on each side of the player have hit something
	pub hits: SideMap<bool>,
}
impl WallSensor {
	pub fn at_offset(local_offset: f32) -> Self {
		Self {
			local_offset,
			hits: default(),
		}
	}
}

/// A set of four [WallSensor]s.
///
/// As a collective, the sensors can be used not only to detect obstacles adjacent
/// to the associated player entity, but also to distinguish wall-like obstacles
/// from other things like ledges or steps.
///
/// The `Default` instance will initialize the four sensors at local height offsets
/// `[1/8, 3/8, 5/8, 7/8]`, i.e. equidistant to each other, with some space apart
/// from the top and bottom of the collider.
#[derive(Debug)]
pub struct WallSensors([WallSensor; 4]);

impl Default for WallSensors {
	fn default() -> Self {
		let gap = 0.25;
		let bottom_height = gap * 0.5;
		WallSensors([
			WallSensor::at_offset(bottom_height),
			WallSensor::at_offset(bottom_height + gap),
			WallSensor::at_offset(bottom_height + gap * 2.0),
			WallSensor::at_offset(bottom_height + gap * 3.0),
		])
	}
}
impl WallSensors {
	/// Updates the `hits` state of each sensor in this group by performing ray-casts in the given
	/// `rapier_context`, with edges of the rectangular "player" defined in terms of its `center`
	/// and `half_extents` values.
	pub fn update(
		&mut self,
		center: Vec2,
		half_extents: Vec2,
		ray_length: f32,
		rapier_context: &RapierContext,
		excluded_entity: Entity,
	) {
		let bottom_y = center.y - half_extents.y;
		let height = half_extents.y * 2.0;
		for sensor in &mut self.0 {
			let sensor_y = bottom_y + height * sensor.local_offset;

			for side in Side::BOTH {
				let x_offset = half_extents.x * side;
				let direction = Vec2::X * side;
				let raycast_start = Vec2::new(center.x + x_offset, sensor_y);
				sensor.hits[side] = rapier_context
					.cast_ray(
						/* origin */ raycast_start,
						/* ray_dir */ direction,
						/* max_toi */ ray_length,
						/* solid */ true, // IDK what this means
						/* filter */
						QueryFilter {
							flags: QueryFilterFlags::EXCLUDE_DYNAMIC | QueryFilterFlags::EXCLUDE_SENSORS,
							exclude_collider: Some(excluded_entity),
							exclude_rigid_body: Some(excluded_entity),
							..default()
						},
					)
					.is_some();
			}
		}
	}

	/// Uses the given `gizmos` do draw each of the rays that would be cast during `update`
	pub fn draw(&mut self, center: Vec2, half_extents: Vec2, gizmos: &mut Gizmos) {
		let bottom_y = center.y - half_extents.y;
		let height = half_extents.y * 2.0;
		for sensor in &self.0 {
			let sensor_y = bottom_y + height * sensor.local_offset;
			for side in Side::BOTH {
				let x_offset = half_extents.x * side;
				let direction = Vec2::X * side * 0.25;
				let raycast_start = Vec2::new(center.x + x_offset, sensor_y);
				let color = if sensor.hits[side] {
					Color::srgb(0.8, 0.5, 0.0)
				} else {
					Color::srgb(0., 0., 1.)
				};
				gizmos.ray_2d(raycast_start, direction, color);
			}
		}
	}

	/// Interprets the current `hits` state of the sensor group, to determine whether there is
	/// a wall (or something else) on the requested `side`.
	pub fn interpret(&self, side: Side) -> WallSensorResult {
		// make a 4-bit number to represent the wall sensors, where the least-significant bit
		// represents the bottom sensor, and the bit is 1 when its respective sensor was "hit"
		let mut hit_flags = 0u8;
		for (i, hit) in self.0.iter().map(|s| s.hits[side]).enumerate() {
			if hit {
				hit_flags |= 1 << i;
			}
		}
		match hit_flags {
			0b0001 => WallSensorResult::Step,
			0b0011 => WallSensorResult::Ledge,
			0b0111 => WallSensorResult::Wall,
			0b1111 => WallSensorResult::Wall,
			0b1110 => WallSensorResult::Wall,
			_ => WallSensorResult::NotAWall,
		}
	}
}

/// A sensor-based interpretation of a wall, as decided by [WallSensors::interpret]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum WallSensorResult {
	/// Empty space, or a small obstacle that doesn't seem to be a wall
	NotAWall,

	/// A small obstacle that only impedes the lower quarter of the player,
	/// for example a stair that the player could step onto.
	Step,

	/// A medium obstacle that only impedes the lower half of the player,
	/// for example a ledge that the player could climb onto.
	Ledge,

	/// A large obstacle that impedes most or all of the player,
	/// which could be grabbed or climbed.
	Wall,
}
