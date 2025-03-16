use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::{default, Entity, Gizmos};
use bevy_rapier2d::pipeline::{QueryFilter, QueryFilterFlags};
use bevy_rapier2d::plugin::RapierContext;
use crate::util::{Side, SideMap};

#[derive(Default)]
pub struct WallGrabState {
	/// What the player is *currently* holding onto
	pub current_grabbed: Option<Side>,

	/// What the player *most recently* was holding onto.
	/// When `current_grabbed` is `Some`, `latest_grabbed == current_grabbed`
	latest_grabbed: Option<Side>,

	/// Number of frames (capped at 255) since the player let go of the `latest_grabbed` wall.
	/// This value should be ignored when `latest_grabbed` is `None`.
	time_since_let_go: u8,
}

impl WallGrabState {
	/// Advances the state by one frame.
	pub fn tick(&mut self, current_grabbed: Option<Side>) {
		match current_grabbed {
			None => {
				if self.current_grabbed.is_some() {
					// player just let go now
					self.time_since_let_go = 0;
					self.latest_grabbed = self.current_grabbed;
				} else {
					// player had previously let go
					self.time_since_let_go = self.time_since_let_go.saturating_add(1);
				}
				self.current_grabbed = None;
			}
			Some(side) => {
				self.latest_grabbed = Some(side);
				self.current_grabbed = Some(side);
				self.time_since_let_go = 0;
			}
		}
	}
	
	/// Accessor for `latest_grabbed` that instead returns `None` if the grab
	/// ended more than `coyote_time` frames ago.
	pub fn latest_grabbed_within(&self, coyote_time: u8) -> Option<Side> {
		if self.time_since_let_go <= coyote_time {
			self.latest_grabbed
		} else {
			None
		}
	}
}

/// Describes a sensor that exists at the sides of a player's collider,
/// projecting rays to each side to detect walls in a physics world.
#[derive(Default, Debug)]
pub struct WallGrabSensor {
	/// Ratio value between 0.0 and 1.0 representing how far from the bottom of the
	/// player's collider this sensor exists
	local_offset: f32,
	
	/// Tracks whether the ray-casts on each side of the player have hit something
	pub hits: SideMap<bool>,
}
impl WallGrabSensor {
	pub fn at_offset(local_offset: f32) -> Self {
		Self {
			local_offset,
			hits: default(),
		}
	}
}

/// A set of four `WallGrabSensor`s.
/// 
/// As a collective, the sensors can be used not only to detect obstacles adjacent
/// to the associated player entity, but also to distinguish wall-like obstacles
/// from other things like ledges or steps.
/// 
/// The `Default` instance will initialize the four sensors at local height offsets
/// `[1/8, 3/8, 5/8, 7/8]`, i.e. equidistant to each other, with some space apart
/// from the top and bottom of the collider.
#[derive(Debug)]
pub struct WallGrabSensors([WallGrabSensor; 4]);

impl Default for WallGrabSensors {
	fn default() -> Self {
		let gap = 0.25;
		let bottom_height = gap * 0.5;
		WallGrabSensors([
			WallGrabSensor::at_offset(bottom_height),
			WallGrabSensor::at_offset(bottom_height + gap),
			WallGrabSensor::at_offset(bottom_height + gap * 2.0),
			WallGrabSensor::at_offset(bottom_height + gap * 3.0),
		])
	}
}
impl WallGrabSensors {
	/// Updates the `hits` state of each sensor in this group by performing ray-casts in the given
	/// `rapier_context`, with edges of the rectangular "player" defined in terms of its `center`
	/// and `half_extents` values.
	pub fn update(&mut self, center: Vec2, half_extents: Vec2, rapier_context: &RapierContext, excluded_entity: Entity) {
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
						/* max_toi */ 1.0,
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
				let direction = Vec2::X * side;
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
	pub fn interpret(&self, side: Side) -> WallInterpretation {
		// make a 4-bit number to represent the wall sensors, where the least-significant bit
		// represents the bottom sensor, and the bit is 1 when its respective sensor was "hit"
		let mut hit_flags = 0u8;
		for (i, hit) in self.0.iter().map(|s| s.hits[side]).enumerate() {
			if hit {
				hit_flags |= 1 << i;
			}
		}
		match hit_flags {
			0b0001 => WallInterpretation::Step,
			0b0011 => WallInterpretation::Ledge,
			0b0111 => WallInterpretation::Wall,
			0b1111 => WallInterpretation::Wall,
			0b1110 => WallInterpretation::Wall,
			_ => WallInterpretation::NotAWall,
		}
	}
}

/// A sensor-based interpretation of a wall, as decided by [WallGrabSensors::interpret]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum WallInterpretation {
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