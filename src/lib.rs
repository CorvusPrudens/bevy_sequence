use bevy::{app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*};

pub mod evaluate;
pub mod fragment;

mod fragment2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct FragmentId(Entity);

impl FragmentId {
    pub fn new(fragment: Entity) -> Self {
        Self(fragment)
    }

    pub fn end(&self) -> FragmentEndEvent {
        FragmentEndEvent { id: *self }
    }
}

/// The full path of IDs for a particular event.
///
/// The path cannot be constructed without at least one
/// ID, so the path can always produce a leaf ID.
///
/// Since fragments are organized in a tree structure,
/// this path provides information for the entire branch.
#[derive(Debug, Clone, Component)]
pub struct IdPath(Vec<FragmentId>);

impl AsRef<[FragmentId]> for IdPath {
    fn as_ref(&self) -> &[FragmentId] {
        &self.0
    }
}

impl core::ops::Deref for IdPath {
    type Target = [FragmentId];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(dead_code)]
impl IdPath {
    pub fn new(path: Vec<FragmentId>) -> Self {
        assert!(!path.is_empty(), "An ID path must have at least one node.");

        Self(path)
    }

    pub fn leaf(&self) -> &FragmentId {
        self.first().unwrap()
    }
}

#[derive(Debug, Event, Clone)]
pub struct FragmentEvent<Data> {
    pub id: FragmentId,
    pub data: Data,
}

#[allow(unused)]
impl<D> FragmentEvent<D> {
    pub fn end(&self) -> FragmentEndEvent {
        FragmentEndEvent { id: self.id }
    }
}

#[derive(Debug, Event)]
pub struct FragmentEndEvent {
    pub id: FragmentId,
}

pub struct SequencePlugin;

impl Plugin for SequencePlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(FragmentUpdate);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_before(PreUpdate, FragmentUpdate);

        app.insert_resource(evaluate::EvaluatedFragments::default())
            .insert_resource(evaluate::FragmentStates::default())
            .add_event::<FragmentEndEvent>()
            .add_systems(
                FragmentUpdate,
                (
                    fragment::update_sequence_items,
                    fragment::update_limit_items,
                    fragment::update_all_items,
                )
                    .in_set(EvaluateSet),
            )
            .add_systems(PreUpdate, fragment::manage_delay);
    }
}

/// Schedule to emit events from fragments to ensure visibility in a bevy update schedules.
#[derive(ScheduleLabel, Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct FragmentUpdate;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct EvaluateSet;
