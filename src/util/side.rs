use std::ops::{Index, IndexMut, Mul, Neg};
use bevy::prelude::Vec2;

/// Left or Right enum
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Side {
	Left,
	Right,
}

impl Neg for Side {
	type Output = Side;

	fn neg(self) -> Self::Output {
		match self {
			Side::Left => Side::Right,
			Side::Right => Side::Left,
		}
	}
}

impl Side {
	pub const BOTH: [Side; 2] = [Side::Left, Side::Right];
}

macro_rules! impl_side_traits {
	($T:ty, $one:expr) => {
		impl From<Side> for $T {
			fn from(side: Side) -> $T {
				match side {
					Side::Left => -$one,
					Side::Right => $one,
				}
			}
		}
		impl Mul<Side> for $T {
			type Output = Self;
			fn mul(self, side: Side) -> Self::Output {
				match side {
					Side::Left => -self,
					Side::Right => self,
				}
			}
		}
	};
}
impl_side_traits!(f32, 1.0);
impl_side_traits!(f64, 1.0);
impl_side_traits!(i8, 1);
impl_side_traits!(Vec2, Vec2::X);

/// Arbitrary container that holds a value for both `left` and `right`
#[derive(Default, Debug)]
pub struct SideMap<A> {
	pub left: A,
	pub right: A,
}

impl<A> Index<Side> for SideMap<A> {
	type Output = A;

	fn index(&self, index: Side) -> &Self::Output {
		match index {
			Side::Left => &self.left,
			Side::Right => &self.right,
		}
	}
}
impl<A> IndexMut<Side> for SideMap<A> {
	fn index_mut(&mut self, index: Side) -> &mut Self::Output {
		match index {
			Side::Left => &mut self.left,
			Side::Right => &mut self.right,
		}
	}
}