mod control_params;
mod control_state;
mod loader;
mod system;

use bevy::asset::Handle;
use bevy::prelude::Component;
pub use control_params::*;
pub use control_state::*;
pub use loader::*;
pub use system::*;

#[derive(Component, Debug)]
#[require(PlayerControlState)]
pub struct Player(pub Handle<PlayerControlParams>);
