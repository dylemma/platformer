use bevy::math::Vec2;
use bevy::prelude::{Component, Curve, EasingCurve};
use crate::player::ForceDecayCurve;
use crate::util::{CapacitiveFlag, Cooldown, FrameCount, PlayerWallControlState, Side, WallSensors};

#[derive(Component, Default)]
pub struct PlayerControlState {
    /// tracks whether the player is on the ground, or how recently they were on the ground
    pub grounded: CapacitiveFlag,

    /// tracks whether the player is airborne as the result of a jump (as opposed to falling)
    pub jumping: bool,

    pub x_when_jumped: Option<f32>,
    pub y_when_jumped: Option<f32>,

    /// flag used to avoid decrementing `jumps_remaining` every frame while in midair
    pub lost_jump_due_to_falling: bool,

    /// Velocity derived from the directional inputs, and gravity.
    /// Does not exclude "external forces" affecting the X direction, like
    /// wall jumping or moving platforms
    pub own_velocity: Vec2,

    /// Input buffer for jumping
    pub jump_requested: CapacitiveFlag,

    /// Resource counter for the player's jumps. Decrements when jumping from the ground or falling
    /// from a platform. Resets when landing on the ground
    pub jumps_remaining: u8,

    /// cooldown timer for jumping
    pub jump_cooldown: Cooldown,

    /// a sensor object used to detect walls, ledges, and steps adjacent to the player
    pub wall_sensors: WallSensors,

    /// a decaying force that is added when wall-jumping
    pub wall_jump_force: TemporaryForce,

    /// amount of time after wall jumping, where attempting to move back towards the wall will be ignored
    pub wall_jump_input_cooldown: Cooldown,

    /// tracks the side that the wall was on, when jumping from it
    pub wall_jump_latest_side: Option<Side>,

    /// state that becomes active when the player comes in contact with a wall while airborne
    pub wall_control_state: PlayerWallControlState,

    /// remembers the total computed velocity (per-second) from the previous update
    pub previous_total_velocity: Vec2,
}


#[derive(Default)]
pub struct TemporaryForce {
    pub age: FrameCount,
    pub max: Vec2,
}

impl TemporaryForce {
    pub fn eval(&self, curve: &ForceDecayCurve) -> Vec2 {
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
    pub fn tick(&mut self) {
        self.age.increment();
    }
    pub fn reset(&mut self, max: Vec2) {
        self.max = max;
        self.age = FrameCount(0);
    }
}