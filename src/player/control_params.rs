use crate::util::{FrameCount, PlayerWallControlParams};
use bevy::prelude::{Asset, Component, EaseFunction, TypePath};
use serde::Deserialize;

#[derive(Asset, Copy, Clone, Component, Debug, Deserialize, TypePath)]
pub struct PlayerControlParams {
	pub run: HorizontalControlParams,
	pub float: HorizontalControlParams,
	pub jump_speed: f32,
	pub gravity: f32,
	pub coyote_time: FrameCount,
	pub jump_input_buffer: FrameCount,
	pub max_jumps: u8,
	pub jump_cooldown: FrameCount,
	pub wall_jump_force_decay: ForceDecayCurve,
	pub wall_jump_input_cooldown: FrameCount,
	pub wall_control_params: PlayerWallControlParams,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct HorizontalControlParams {
	pub max_speed: f32,
	pub acceleration: f32,
	pub deceleration: f32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct ForceDecayCurve {
	pub easing: EaseFunction,
	pub duration: FrameCount,
}
