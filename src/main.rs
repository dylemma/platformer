use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, LoadState};
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy_rapier2d::prelude::*;
use serde::Deserialize;
use thiserror::Error;

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
        // .add_systems(FixedUpdate, read_result_system.after(player_system))
        //
        // custom asset loader test
        //
        .init_asset::<MyCustomAsset>()
        .init_asset_loader::<MyCustomAssetLoader>()
        .add_systems(Startup, setup_assets)
        .add_systems(Update, watch_assets)
        .init_resource::<AssetLoadingState>()
        //
        // rapier physics
        //
        .insert_resource(TimestepMode::Fixed {
            dt: 1. / 60.,
            substeps: 1,
        })
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(10.0).in_fixed_schedule())
        .add_plugins(RapierDebugRenderPlugin::default())
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

struct WallArgs {
    color: Color,
    pos: Vec2,
    size: Vec2,
}
impl WallArgs {
    fn spawn(self, commands: &mut Commands) {
        let WallArgs { color, pos, size } = self;
        commands.spawn((
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
        pos: Vec2::new(75.0, 20.0),
        size: Vec2::new(20.0, 2.0),
    }
    .spawn(&mut commands);

    // platform 2
    WallArgs {
        color: Color::srgb(0.15, 0.8, 0.25),
        pos: Vec2::new(50.0, 35.0),
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
#[require(PlayerControlState)]
struct Player {
    max_run_speed: f32,
    run_acceleration: f32,
    run_deceleration: f32,
    jump_speed: f32,
    gravity: f32,
    coyote_time_frames: u32,
    jump_buffer_frames: u32,
    max_jumps: u8,
    jump_cooldown_frames: u8,
    // air_max_speed: f32,
    // air_acceleration: f32,
    // air_deceleration: f32,
    // current_air_speed: f32,
}

#[derive(Component, Default)]
struct PlayerControlState {
    grounded: bool,
    jumping: bool,
    lost_jump_due_to_falling: bool,
    current_run_speed: f32,
    current_fall_speed: f32, // TODO: naming is kinda backwards because positive means "going up"
    frames_since_grounded: u32,
    frames_since_jump_input: u32,
    jumps_remaining: u8,
    frames_since_jumped: u8,
}

#[derive(Component)]
struct PlayerStatusText;

fn setup_player(mut commands: Commands) {
    commands.spawn((
        Player {
            max_run_speed: 90.0,
            // accelerate to max speed in a quarter second (15 frames)
            run_acceleration: 6.0,
            // decelerate from max speed to 0 in 1/10 second (6 frames)
            run_deceleration: 15.0,
            jump_speed: 150.0,
            gravity: -10.0,
            coyote_time_frames: 6,
            jump_buffer_frames: 6,
            max_jumps: 1, // can be set to 2, to allow double-jump
            jump_cooldown_frames: 8,
            // air_max_speed: 25.0,
            // air_acceleration: 5.0,
            // air_deceleration:
        },
        Friction {
            coefficient: 0.0,
            combine_rule: CoefficientCombineRule::Multiply,
        },
        Sprite::from_color(Color::srgb(1., 0.5, 0.), Vec2::new(5.0, 5.0)),
        Collider::cuboid(2.5, 2.5),
        Transform::from_xyz(25., 25., 0.),
        RigidBody::KinematicPositionBased,
        KinematicCharacterController::default(),
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
        &Player,
        &mut PlayerControlState,
        &mut KinematicCharacterController,
        &KinematicCharacterControllerOutput,
    )>,
    mut status_text_query: Query<&mut Text, With<PlayerStatusText>>,
    time: Res<Time>,
) {
    let mut status_text = status_text_query.single_mut();

    let input_axis = {
        let mut v = Vec2::new(0., 0.);
        if kb.pressed(KeyCode::KeyA) {
            v.x -= 1.0;
        }
        if kb.pressed(KeyCode::KeyD) {
            v.x += 1.0;
        }
        if kb.pressed(KeyCode::KeyW) {
            v.y += 1.0;
        }
        if kb.pressed(KeyCode::KeyS) {
            v.y -= 1.0;
        }
        v
    };

    let jump_requested = kb.just_pressed(KeyCode::Space);

    for (player_params, mut player, mut controller, last_controller_out) in &mut player_query {
        // TODO: arrest vertical momentum when hitting ceilings
        for collision in &last_controller_out.collisions {
            info!(
                "player hit {:?} with {:?} left",
                collision.entity, collision.translation_remaining
            );
        }

        player.current_run_speed = compute_run_velocity(
            /* current */ player.current_run_speed,
            /* desired */ input_axis.x * player_params.max_run_speed,
            /* accel  */ player_params.run_acceleration,
            /* decel  */ player_params.run_deceleration,
        );

        player.grounded = last_controller_out.grounded;

        // refund jump ability when reaching the ground
        if player.grounded {
            player.jumps_remaining = player_params.max_jumps;
            player.jumping = false;
            player.lost_jump_due_to_falling = false;
        }

        // coyote timer
        if player.grounded {
            player.frames_since_grounded = 0;
        } else {
            player.frames_since_grounded = player.frames_since_grounded.saturating_add(1);
        }

        // if player walks off a platform without jumping, then they lose a jump
        if player.frames_since_grounded > player_params.coyote_time_frames {
            if !player.lost_jump_due_to_falling && !player.jumping {
                player.jumps_remaining = player.jumps_remaining.saturating_sub(1);
                player.lost_jump_due_to_falling = true;
            }
        }

        // jump-buffering
        if jump_requested {
            player.frames_since_jump_input = 0;
        } else {
            player.frames_since_jump_input = player.frames_since_jump_input.saturating_add(1);
        }

        // manage jump cooldown (more important when double-jump is enabled)
        player.frames_since_jumped = player.frames_since_jumped.saturating_add(1);

        // apply gravity
        if player.grounded {
            player.current_fall_speed = 0.0;
        } else {
            player.current_fall_speed += player_params.gravity;
        }

        // jump
        let wants_to_jump = player.frames_since_jump_input <= player_params.jump_buffer_frames;
        let can_jump = player.jumps_remaining > 0
            && player.frames_since_jumped > player_params.jump_cooldown_frames;

        if wants_to_jump && can_jump {
            info!("jumping with coyote time {}", player.frames_since_grounded);
            player.current_fall_speed = player_params.jump_speed;
            player.jumps_remaining -= 1;
            player.jumping = true;
            player.frames_since_jumped = 0;
        }

        // finish velocity computation
        let player_velocity_per_sec =
            Vec2::new(player.current_run_speed, player.current_fall_speed);

        // debug text for velocity
        status_text.0 = format!(
            "vx: {}, vy: {}\ngrounded: {}\njumps: {}",
            player_velocity_per_sec.x,
            player_velocity_per_sec.y,
            player.grounded,
            player.jumps_remaining,
        );

        // send computed translation to controller for resolution in the physics world
        controller.translation = Some(player_velocity_per_sec * time.delta_secs());
    }
}

/// Solve for a player's new horizontal velocity, by accelerating or decelerating
/// their current velocity towards their desired velocity
fn compute_run_velocity(
    current_vel: f32,
    desired_vel: f32,
    acceleration: f32,
    deceleration: f32,
) -> f32 {
    // The player will have a separate "speed up" and "slow down" rate;
    // Pick the appropriate one based on the difference between current
    // and desired velocities.
    let accel_base = if desired_vel == 0.0 {
        // if the goal is to stop, that's always deceleration
        deceleration
    } else if desired_vel.signum() != current_vel.signum() {
        // if the goal is in the opposite direction, decelerate to 0 first
        deceleration
    } else if desired_vel.abs() < current_vel.abs() {
        // same direction but goal speed is slower; decelerate
        deceleration
    } else {
        // gotta go fast!
        acceleration
    };

    // What's the overall change in velocity the player wants to achieve?
    let goal_delta = desired_vel - current_vel;

    // If the acceleration is more than enough to reach the goal this frame,
    // do so and skip some math
    if goal_delta.abs() < accel_base {
        desired_vel
    } else {
        // Apply acceleration in the direction of the goal
        let accel_amount = accel_base * goal_delta.signum();
        current_vel + accel_amount
    }
}

#[derive(Asset, TypePath, Deserialize, Debug)]
struct MyCustomAsset {
    #[allow(unused)]
    name: String,
}

#[derive(Default)]
struct MyCustomAssetLoader;

#[derive(Debug, Error)]
enum MyCustomAssetLoaderError {
    #[error("Could not load file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Could not parse RON: {0}")]
    RonError(#[from] ron::error::SpannedError),
}

impl AssetLoader for MyCustomAssetLoader {
    type Asset = MyCustomAsset;
    type Settings = ();
    type Error = MyCustomAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        _: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let custom_asset = ron::de::from_bytes::<MyCustomAsset>(&bytes)?;
        Ok(custom_asset)
    }
}

#[derive(Resource, Default)]
struct AssetLoadingState {
    example: Handle<MyCustomAsset>,
    finished: bool,
}

fn setup_assets(mut state: ResMut<AssetLoadingState>, asset_server: Res<AssetServer>) {
    // AssetPlugin uses "assets/" as the base path by default
    state.example = asset_server.load::<MyCustomAsset>("example.ron");
}

fn watch_assets(
    mut state: ResMut<AssetLoadingState>,
    custom_assets: Res<Assets<MyCustomAsset>>,
    srv: Res<AssetServer>,
) {
    let example = custom_assets.get(&state.example);
    if !state.finished {
        match example {
            None => {
                info!("still loading example");
                match srv.load_state(&state.example) {
                    LoadState::Failed(err) => {
                        error!("Failed to  load example: {}", err);
                        state.finished = true;
                    }
                    _ => (),
                }
            }
            Some(loaded) => {
                info!("finished loading example: {:?}", loaded);
                state.finished = true;
            }
        }
    }
}
