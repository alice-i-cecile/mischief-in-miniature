//! A 3rd person platformer camera system.
//!
//! The camera follows the player character smoothly from behind / above.

use bevy::prelude::*;

use crate::characters::Player;

pub(super) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (set_target, move_camera).chain());
    }
}

// The offset from the player character's position in local coordinates.
const OFFSET: Vec3 = Vec3::new(0., 10., 10.);

/// Setup the initial scene
fn setup(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
        CameraTarget::default(),
    ));
}

/// The position that the camera should be in.
#[derive(Component, Default)]
struct CameraTarget {
    /// The position of the camera in absolute world coordinates.
    camera_position: Vec3,
    /// The position of the camera's focus point in absolute world coordinates.
    ///
    /// Usually this is the player character's position.
    focus_position: Vec3,
}

/// Sets where the camera should be, following the player character from behind / above.
fn set_target(
    player: Single<&GlobalTransform, With<Player>>,
    mut camera_target: Single<&mut CameraTarget>,
) {
    let player_transform = player.into_inner().compute_transform();

    let player_rotation = player_transform.rotation;
    // The camera should always be behind the player character,
    // so we rotate the offset by the player's rotation.
    let rotated_offset = player_rotation.mul_vec3(OFFSET);

    camera_target.camera_position = player_transform.translation + rotated_offset;
    camera_target.focus_position = player_transform.translation;
}

/// Moves the camera towards the target position.
fn move_camera(time: Res<Time>, camera: Single<(&mut Transform, &CameraTarget)>) {
    /// Controls how fast the camera moves towards the target position.
    /// Higher values make the camera move faster.
    const DECAY_RATE: f32 = 3.;

    let (mut camera_transform, camera_target) = camera.into_inner();

    // Move the camera towards the target position
    // The smooth_nudge call is timestep invariant!
    camera_transform.translation.smooth_nudge(
        &camera_target.camera_position,
        DECAY_RATE,
        time.delta_secs(),
    );

    // Look at the focus position.
    camera_transform.look_at(camera_target.focus_position, Vec3::Y);
}
