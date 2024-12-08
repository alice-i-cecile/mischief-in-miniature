//! A Katamari-inspired 3D action game where you play a toy that's come to life, causing chaos.

mod camera;
mod characters;
mod pausing;

use avian3d::prelude::*;
use bevy::prelude::*;
use camera::CameraPlugin;
use characters::character_controller::{CharacterController, CharacterControllerPlugin};
use pausing::PausePlugin;

fn main() {
    App::new()
        .add_plugins((
            // Bevy
            DefaultPlugins,
            // Avian
            PhysicsPlugins::default(),
            CameraPlugin,
            CharacterControllerPlugin,
            PausePlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

/// Setup the initial scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    // Player
    commands.spawn((
        CharacterController,
        SceneRoot(assets.load("player.glb#Scene0")),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Collider::capsule(0.2, 0.8),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(2.0),
    ));

    // A cube to move around
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(3.0, 2.0, 3.0),
    ));

    // Environment (see the `collider_constructors` example for creating colliders from scenes)
    commands.spawn((
        SceneRoot(assets.load("character_controller_demo.glb#Scene0")),
        Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::PI * 0.5)),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
        RigidBody::Static,
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            range: 50.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 15.0, 0.0),
    ));
}
