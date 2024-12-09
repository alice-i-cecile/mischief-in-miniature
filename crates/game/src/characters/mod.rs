//! Logic and data for characters in the game.
//!
//! This includes both the player and any non-player agents.

use bevy::prelude::*;

pub(crate) mod character_controller;

/// The marker component for the player character.
#[derive(Component)]
pub(crate) struct Player;
