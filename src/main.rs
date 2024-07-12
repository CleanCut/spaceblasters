use std::os::unix::process;

use bevy::{
    math::VectorSpace,
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    utils::HashMap,
};
use bevy_rapier2d::{prelude::*, rapier::crossbeam::thread};
use leafwing_input_manager::prelude::*;
use rand::{thread_rng, Rng};

fn main() {
    let player_colors = vec![
        Color::srgb(1.0, 1.0, 0.0),
        Color::srgb(0.0, 0.0, 1.0),
        Color::srgb(0.0, 1.0, 0.0),
        Color::srgb(1.0, 0.0, 0.0),
    ];
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(200.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(InputManagerPlugin::<Action>::default())
        .insert_resource(JoinedPlayers(HashMap::new()))
        .insert_resource(PlayerColors(player_colors))
        .add_systems(Startup, setup)
        .add_systems(Update, (join, process_input, wraparound))
        .run();
}

fn wraparound(mut query: Query<&mut Transform>) {
    for mut transform in query.iter_mut() {
        if transform.translation.x < -640.0 {
            transform.translation.x = 640.0;
        } else if transform.translation.x > 640.0 {
            transform.translation.x = -640.0;
        }
        if transform.translation.y < -360.0 {
            transform.translation.y = 360.0;
        } else if transform.translation.y > 360.0 {
            transform.translation.y = -360.0;
        }
    }
}

fn process_input(
    mut commands: Commands,
    mut action_query: Query<(
        &ActionState<Action>,
        &mut Player,
        &mut ExternalForce,
        &mut Transform,
        &mut Velocity,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (action, mut player, mut external_force, mut transform, mut velocity) in
        action_query.iter_mut()
    {
        // TURN!!!
        if action.pressed(&Action::Turn) {
            let turn_value = -action.value(&Action::Turn);
            const TURN_POWER: f32 = 5000.0;
            let torque = turn_value * TURN_POWER;
            external_force.torque = torque;
        } else {
            external_force.torque = 0.0;
        }

        // THRUST!!!
        if action.pressed(&Action::Thrust) {
            let thrust_value = action.value(&Action::Thrust);
            const THRUST_POWER: f32 = 250.0;
            let thrust_vector = transform.up().xy() * thrust_value * THRUST_POWER;
            external_force.force = thrust_vector;
        } else {
            external_force.force = Vec2::ZERO;
        }

        // Shoot!!!
        if action.just_pressed(&Action::Shoot) {
            let bullet_linvelocity = transform.up().xy() * 300.0 + velocity.linvel;
            let bullet_velocity = Velocity {
                linvel: bullet_linvelocity,
                angvel: 0.0,
            };
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: Mesh2dHandle(meshes.add(Circle { radius: 4.0 })),
                    material: materials.add(player.color),
                    transform: *transform,
                    ..default()
                },
                RigidBody::Dynamic,
                Friction::new(0.0),
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                Restitution::new(1.5),
                Collider::ball(4.0),
                Sensor,
                bullet_velocity,
                ColliderMassProperties::Mass(0.05),
            ));
        }

        //

        // // WARP!!!
        // if action.just_pressed(&Action::Warp) {
        //     velocity.linvel = Vec2::ZERO;
        //     velocity.angvel = 0.0;
        //     transform.translation = Vec3::new(
        //         thread_rng().gen_range(-600.0..600.0),
        //         thread_rng().gen_range(-300.0..300.0),
        //         0.0,
        //     );
        // }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    commands.spawn(Camera2dBundle::default());
    rapier_config.gravity = Vec2::ZERO;
    // commands.spawn(SpriteBundle {
    //     texture: asset_server.load(""),
    //     ..Default::default()
    // });
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
enum Action {
    Turn,
    Thrust,
    Shoot,
    Warp,
}

#[derive(Resource)]
struct JoinedPlayers(pub HashMap<Gamepad, Entity>);

#[derive(Component)]
struct Player {
    gamepad: Gamepad,
    color: Color,
}

#[derive(Resource)]
struct PlayerColors(pub Vec<Color>);

fn join(
    mut commands: Commands,
    mut joined_players: ResMut<JoinedPlayers>,
    gamepads: Res<Gamepads>,
    button_inputs: Res<ButtonInput<GamepadButton>>,
    mut player_colors: ResMut<PlayerColors>,
    asset_server: Res<AssetServer>,
) {
    for gamepad in gamepads.iter() {
        if !button_inputs.all_pressed([
            GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger),
            GamepadButton::new(gamepad, GamepadButtonType::RightTrigger),
        ]) {
            continue;
        }
        if joined_players.0.contains_key(&gamepad) {
            continue;
        }
        println!("Gamepad {} has joined the game!", gamepad.id);

        let input_map = InputMap::default()
            .insert(
                Action::Turn,
                SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.1),
            )
            .insert(Action::Shoot, GamepadButtonType::East)
            .insert(Action::Warp, GamepadButtonType::North)
            .insert(Action::Thrust, GamepadButtonType::LeftTrigger2)
            .set_gamepad(gamepad)
            .build();

        let color = player_colors.0.pop().unwrap();
        let player = commands
            .spawn((
                InputManagerBundle::with_map(input_map),
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(
                        thread_rng().gen_range(-600.0..600.0),
                        -250.0,
                        2.0,
                    ),
                    texture: asset_server.load("ship1.png"),
                    ..Default::default()
                },
                Player { gamepad, color },
                RigidBody::Dynamic,
                Friction::new(0.0),
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 8.0,
                },
                Restitution::new(1.5),
                Collider::ball(22.0),
                ExternalImpulse::default(),
                ExternalForce::default(),
                Velocity::default(),
                ColliderMassProperties::Mass(1.0),
            ))
            .id();

        joined_players.0.insert(gamepad, player);
    }
}
