/// Integer value representing a number of frames, or game "ticks".
/// Can be used to represent a duration, or act as a timer that counts up or down.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct FrameCount(pub usize);

impl FrameCount {
	/// Add one to this counter (will saturate to [usize::MAX])
	pub fn increment(&mut self) {
		self.0 = self.0.saturating_add(1);
	}

	/// Remove one from this counter (will saturate at 0)
	pub fn decrement(&mut self) {
		self.0 = self.0.saturating_sub(1);
	}
	
	/// Clear the counter, setting is value to 0
	pub fn reset(&mut self) {
		self.0 = 0;
	}
}

/// A cooldown timer.
/// By default, the timer is "ready".
/// The timer can be `reset` to a specified duration, so that it will not be ready again
/// until [Cooldown::tick] is called the specified number of times.
#[derive(Default, Debug)]
pub struct Cooldown(FrameCount);

impl Cooldown {
	/// Put the associated action "on cooldown" for the specified duration
	pub fn reset(&mut self, duration: FrameCount) {
		self.0 = duration;
	}

	/// Advance the cooldown timer, possibly causing the associated action to become "ready"
	pub fn tick(&mut self) {
		self.0.decrement();
	}

	/// Check if the associated action is ready, i.e. "off cooldown"
	pub fn is_ready(&self) -> bool {
		self.0 == FrameCount(0)
	}
}

/// A boolean flag that remembers how long it has been un-set.
/// Used for coyote time and jump buffering.
#[derive(Debug)]
pub struct CapacitiveFlag {
	value: bool,
	time_since_released: FrameCount,
}

impl Default for CapacitiveFlag {
	fn default() -> Self {
		Self {
			value: false,
			time_since_released: FrameCount(usize::MAX),
		}
	}
}

impl CapacitiveFlag {
	/// Set or clear the flag, incrementing the internal timer when the flag remains cleared
	pub fn tick(&mut self, value: bool) {
		if value {
			// doesn't matter if it just became true, or continued to be true;
			// we always reset the timer to 0 since the flag is not "released"
			self.value = true;
			self.time_since_released = FrameCount(0);
		} else {
			// if the flag was already false, this is a subsequent frame since
			// the initial "release", so we increment the frame counter
			if !self.value {
				self.time_since_released.increment();
			}
			self.value = false;
		}
	}

	/// Check if the flag is *currently* set
	pub fn is_set(&self) -> bool {
		self.value
	}

	/// Check if the flag is currently set, or has been set at any time in the last `duration`.
	/// For example, "did the player try to jump within the last 3 frames?"
	pub fn was_set_within(&self, duration: FrameCount) -> bool {
		self.time_since_released <= duration
	}
}
