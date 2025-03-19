use bevy::prelude::Vec2;
use std::ops::{Index, IndexMut, Mul, Neg};

// Defines `enum Side` and `struct SideMap<A>` for left/right values,
// and `enum YSide` and `struct YSideMap<A>` for up/down values.
macro_rules! impl_sidemap_index {
	($XorY:ident, $Self:ident, $Map:ident, $Pos:ident => $pos:ident, $Neg:ident => $neg:ident) => {
		#[derive(Debug, Eq, PartialEq, Copy, Clone)]
		pub enum $Self {
			$Pos,
			$Neg,
		}
		
		#[derive(Default, Debug)]
		pub struct $Map<A> {
			pub $pos: A,
			pub $neg: A,
		}
		
		impl $Self {
			#[allow(unused)]
			pub const BOTH: [$Self; 2] = [<$Self>::$Pos, <$Self>::$Neg];
		}
		
		impl <A> Index<$Self> for $Map<A> {
			type Output = A;
			fn index(&self, side: $Self) -> &Self::Output {
				match side {
					<$Self>::$Pos => &self.$pos,
					<$Self>::$Neg => &self.$neg,
				}
			}
		}
		impl <A> IndexMut<$Self> for $Map<A> {
			fn index_mut(&mut self, side: $Self) -> &mut Self::Output {
				match side {
					<$Self>::$Pos => &mut self.$pos,
					<$Self>::$Neg => &mut self.$neg,
				}
			}
		}
		
		impl Neg for $Self {
			type Output = $Self;
			fn neg(self) -> Self::Output {
				match self {
					<$Self>::$Neg => <$Self>::$Pos,
					<$Self>::$Pos => <$Self>::$Neg,
				}
			}
		}
		
		macro_rules! impl_extra_traits {
			($T:ty, $one:expr) => {
				impl From<$Self> for $T {
					fn from(side: $Self) -> $T {
						match side {
							<$Self>::$Pos => $one,
							<$Self>::$Neg => -$one,
						}
					}
				}
				impl Mul<$Self> for $T {
					type Output = Self;
					fn mul(self, side: $Self) -> Self::Output {
						match side {
							<$Self>::$Pos => self,
							<$Self>::$Neg => -self,
						}
					}
				}
			};
		}
		
		impl_extra_traits!(f32, 1.0);
		impl_extra_traits!(f64, 1.0);
		impl_extra_traits!(i8, 1);
		impl_extra_traits!(Vec2, Vec2::$XorY);
	}
}
impl_sidemap_index!(X, Side, SideMap, Right => right, Left => left);
impl_sidemap_index!(Y, YSide, YSideMap, Up => up, Down => down);