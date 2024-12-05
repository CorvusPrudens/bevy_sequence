use bevy_ecs::prelude::*;

pub mod evaluate;
// pub mod fragment;
pub mod fragment2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct FragmentId(Entity);

impl FragmentId {
    pub fn new(fragment: Entity) -> Self {
        Self(fragment)
    }
}

pub trait Threaded: Send + Sync + 'static {}

impl<T> Threaded for T where T: Send + Sync + 'static {}
