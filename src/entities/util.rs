use bevy::prelude::*;

pub const ENABLE_WIREFRAME: bool = false;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Lifetime {
    pub timer: Timer
}