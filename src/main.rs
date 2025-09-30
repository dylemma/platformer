mod player;
mod util;

use crate::player::{Player, PlayerAssetLoader, PlayerControlParams, player_system};
use bevy::asset::AssetServer;
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy_rapier2d::prelude::*;

fn main() {
	App::new()
		// baseline bevy stuff
		.add_plugins(DefaultPlugins.set(AssetPlugin {
			// opt in to hot reloading of assets
			watch_for_changes_override: Some(true),
			..default()
		}))
		.init_asset::<PlayerControlParams>()
		.init_asset_loader::<PlayerAssetLoader>()
		.insert_resource(Time::<Fixed>::from_hz(60.))
		.add_systems(Update, watch_player_config)
		//
		// platformer learning zone
		//
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

#[derive(Component)]
struct PlayerStatusText;

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn((
		Player(asset_server.load("player.ron")),
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

fn watch_player_config(mut events: EventReader<AssetEvent<PlayerControlParams>>) {
	for event in events.read() {
		if let AssetEvent::Modified { id } = event {
			info!("Asset modified: {:?}", id);
		}
	}
}
