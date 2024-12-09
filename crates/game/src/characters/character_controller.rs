//! Movement and controls for playable characters.

use avian3d::prelude::*;
use bevy::{
    ecs::{component::ComponentId, query::Has, world::DeferredWorld},
    prelude::*,
};

use avian3d::math::*;

/// A plugin for character controller logic.
pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>().add_systems(
            Update,
            (
                keyboard_input,
                update_grounded,
                movement,
                apply_movement_damping,
            )
                .chain(),
        );
    }
}

/// An event sent for a movement input action.
#[derive(Event)]
pub enum MovementAction {
    /// Turn to the left
    TurnLeft,
    /// Turn to the right
    TurnRight,
    /// Strafe left
    StrafeLeft,
    /// Strafe right
    StrafeRight,
    /// Move forward
    MoveForward,
    /// Move backward
    MoveBackward,
    /// Jump in the Y direction.
    Jump,
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
#[require(
    Transform,
    Visibility,
    RigidBody,
    Collider,
    ShapeCaster,
    LockedAxes(||LockedAxes::new().lock_rotation_x().lock_rotation_z()),
    Friction,
    Restitution,
    MovementCharacteristics,
)]
#[component(on_add = setup_shapecaster)]
pub struct CharacterController;

/// Override the default shapecaster on spawn to be based on the collider.
fn setup_shapecaster(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    /// The relative size of the shape caster compared to the collider.
    /// Setting this value to less than 1.0 prevents the shape caster from
    /// bouncing on the ground.
    const CAST_SCALE: Scalar = 0.99;

    /// How far out from the collider the shape caster checks for ground.
    /// Setting this value above 0 gives a bit of a buffer to help the controls
    /// feel more responsive.
    const CAST_RADIUS: Scalar = 0.2;

    /// The resolution of the shape caster, in number of subdivisions.
    ///
    /// This is fundamentally a performance vs accuracy tuning knob.
    const CAST_RESOLUTION: u32 = 10;

    let collider = world
        .get::<Collider>(entity)
        .expect("Collider is a required component of CharacterController");

    // Create shape caster as a slightly smaller version of the collider
    let mut caster_shape = collider.clone();
    caster_shape.set_scale(Vector::ONE * CAST_SCALE, CAST_RESOLUTION);

    let ground_caster = ShapeCaster::new(
        caster_shape,
        Vector::ZERO,
        Quaternion::default(),
        Dir3::NEG_Y,
    )
    .with_max_distance(CAST_RADIUS);

    let mut shape_caster = world.get_mut::<ShapeCaster>(entity).unwrap();
    *shape_caster = ground_caster;
}

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

/// The various constants used to define character movement.
///
/// All values should be positive.
#[derive(Component)]
pub struct MovementCharacteristics {
    /// The rate of acceleration when moving forward,
    /// in units per second squared.
    forward: Scalar,
    /// The rate of acceleration when strafing sideways,
    /// in units per second squared.
    strafe: Scalar,
    /// The rate of acceleration when moving backward,
    /// in units per second squared.
    backward: Scalar,
    /// The rate of acceleration when turning,
    /// in radians per second squared.
    turn_speed: Scalar,
    /// The amount of impulse applied when jumping.
    jump_impulse: Scalar,
    /// The maximum angle in radians that a slope can have for a character controller
    /// to be able to climb and jump. If the slope is steeper than this angle,
    /// the character will slide down.
    max_slope_angle: Scalar,
    /// The damping factor for linear movement.
    ///
    /// This should be in the range [0, 1],
    /// and represents the fraction of velocity that is retained each second.
    /// Lower values will cause speed to decay more quickly.
    linear_vel_decay: Scalar,
    /// The damping factor for angular movement.
    ///
    /// This should be in the range [0, 1],
    /// and represents the fraction of velocity that is retained each second.
    /// Lower values will cause speed to decay more quickly.
    angular_vel_decay: Scalar,
}

impl Default for MovementCharacteristics {
    fn default() -> Self {
        Self {
            forward: 50.0,
            strafe: 30.0,
            backward: 30.0,
            turn_speed: 20.0,
            jump_impulse: 7.0,
            max_slope_angle: PI * 0.45,
            linear_vel_decay: 0.9,
            angular_vel_decay: 0.9,
        }
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
// TODO: just use LWIM for this
fn keyboard_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let forward = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let back = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);

    // FIXME: arrow key bindings don't play nice with strafe keys
    let strafe_left = keyboard_input.any_pressed([KeyCode::KeyQ, KeyCode::ArrowLeft]);
    let strafe_right = keyboard_input.any_pressed([KeyCode::KeyE, KeyCode::ArrowRight]);

    let turn_left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let turn_right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let turning = turn_right as i8 - turn_left as i8;
    let horizontal = strafe_right as i8 - strafe_left as i8;
    let vertical = forward as i8 - back as i8;

    match turning {
        1 => {
            movement_event_writer.send(MovementAction::TurnRight);
        }
        -1 => {
            movement_event_writer.send(MovementAction::TurnLeft);
        }
        _ => {}
    };

    match horizontal {
        1 => {
            movement_event_writer.send(MovementAction::StrafeRight);
        }
        -1 => {
            movement_event_writer.send(MovementAction::StrafeLeft);
        }
        _ => {}
    }

    match vertical {
        1 => {
            movement_event_writer.send(MovementAction::MoveForward);
        }
        -1 => {
            movement_event_writer.send(MovementAction::MoveBackward);
        }
        _ => {}
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_event_writer.send(MovementAction::Jump);
    }
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (Entity, &ShapeHits, &Rotation, &MovementCharacteristics),
        With<CharacterController>,
    >,
) {
    for (entity, hits, rotation, movement_characteristics) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits.iter().any(|hit| {
            (rotation * -hit.normal2).angle_between(Vector::Y).abs()
                <= movement_characteristics.max_slope_angle
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

/// Responds to [`MovementAction`] events and moves character controllers accordingly.
fn movement(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<(
        &Transform,
        &MovementCharacteristics,
        &mut LinearVelocity,
        &mut AngularVelocity,
        Has<Grounded>,
    )>,
) {
    // Precision is adjusted so that the example works with
    // both the `f32` and `f64` features. Otherwise you don't need this.
    let delta_time = time.delta_secs_f64().adjust_precision();

    for event in movement_event_reader.read() {
        for (
            transform,
            movement_characteristics,
            mut linear_velocity,
            mut angular_velocity,
            is_grounded,
        ) in &mut controllers
        {
            match event {
                MovementAction::TurnLeft => {
                    angular_velocity.y += movement_characteristics.turn_speed * delta_time;
                }
                MovementAction::TurnRight => {
                    angular_velocity.y -= movement_characteristics.turn_speed * delta_time;
                }
                MovementAction::MoveForward => {
                    linear_velocity.0 +=
                        transform.forward() * movement_characteristics.forward * delta_time;
                }
                MovementAction::MoveBackward => {
                    linear_velocity.0 +=
                        transform.back() * movement_characteristics.backward * delta_time
                }
                MovementAction::StrafeLeft => {
                    linear_velocity.0 +=
                        transform.left() * movement_characteristics.strafe * delta_time;
                }
                MovementAction::StrafeRight => {
                    linear_velocity.0 +=
                        transform.right() * movement_characteristics.strafe * delta_time;
                }
                MovementAction::Jump => {
                    if is_grounded {
                        linear_velocity.y = movement_characteristics.jump_impulse;
                    }
                }
            }
        }
    }
}

/// Slows down movement in the XZ plane.
fn apply_movement_damping(
    mut query: Query<(
        &MovementCharacteristics,
        &mut LinearVelocity,
        &mut AngularVelocity,
    )>,
) {
    for (movement_characteristics, mut linear_velocity, mut angular_velocity) in &mut query {
        // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
        linear_velocity.x *= movement_characteristics.linear_vel_decay;
        linear_velocity.z *= movement_characteristics.linear_vel_decay;
        // Angular damping is done here for consistency
        // All other axis are locked, so we only need to dampen the Y axis
        angular_velocity.y *= movement_characteristics.angular_vel_decay;
    }
}
