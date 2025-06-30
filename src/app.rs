use crate::{
    combinators::CombinatorPlugin,
    fragment::{
        self,
        event::{
            BeginStage, EndStage, MapFn, OnBeginDown, OnBeginUp, OnEndDown, OnEndUp, OnInterruptUp,
        },
    },
    prelude::*,
};
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::ScheduleSystem};
use bevy_platform::collections::HashSet;
use std::any::TypeId;

/// `bevy_sequence`'s plugin.
pub struct SequencePlugin;

/// Sets for every `bevy_sequence` system.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SequenceSets {
    /// Evaluate node scores.
    ///
    /// This set is placed in [PreUpdate].
    Evaluate,

    /// Iterate over all nodes and determine which, if any, should be selected.
    ///
    /// This set is placed in [PreUpdate].
    Select,

    /// Emit events from the selected nodes.
    ///
    /// This set is placed in [PreUpdate].
    Emit,

    /// Respond to end events.
    ///
    /// This set is placed in [PostUpdate].
    Respond,

    /// Save node state.
    ///
    /// This set is placed in [PostUpdate].
    Save,
}

impl Plugin for SequencePlugin {
    fn build(&self, app: &mut App) {
        let world = app.world_mut();
        world.register_component::<OnBeginUp>();
        world.register_component::<OnBeginDown>();
        world.register_component::<OnEndUp>();
        world.register_component::<OnEndDown>();
        world.register_component::<OnInterruptUp>();
        world.register_component::<MapFn<BeginStage>>();
        world.register_component::<MapFn<EndStage>>();

        app.add_plugins(CombinatorPlugin)
            .insert_resource(AddedSystems(Default::default()))
            .insert_resource(fragment::SelectedFragments::default())
            .add_event::<FragmentEndEvent>()
            .add_systems(
                PreUpdate,
                (
                    crate::fragment::select_fragments.in_set(SequenceSets::Select),
                    ApplyDeferred
                        .after(SequenceSets::Evaluate)
                        .before(SequenceSets::Select),
                    crate::fragment::event::begin_world.in_set(SequenceSets::Emit),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    crate::fragment::event::end_world.in_set(SequenceSets::Respond),
                    // crate::fragment::respond_to_leaf.in_set(SequenceSets::Respond),
                    fragment::clear_evals.after(SequenceSets::Respond),
                ),
            )
            .configure_sets(
                PreUpdate,
                (
                    SequenceSets::Select.after(SequenceSets::Evaluate),
                    SequenceSets::Emit.after(SequenceSets::Select),
                ),
            )
            .configure_sets(PostUpdate, SequenceSets::Save.after(SequenceSets::Respond));

        // .add_observer(crate::fragment::event::end_up)
        // .add_observer(crate::fragment::event::begin_up);
    }
}

#[derive(Resource, Default)]
struct AddedSystems(HashSet<TypeId>);

/// Insert systems into a schedule.
///
/// This will only insert a set of systems into a given schedule once.
pub fn add_systems_checked<M, S, C>(world: &mut World, schedule: S, systems: C)
where
    S: ScheduleLabel,
    C: IntoScheduleConfigs<ScheduleSystem, M> + 'static,
{
    let id = TypeId::of::<(S, C)>();
    let mut pairs = world.get_resource_or_insert_with(AddedSystems::default);

    if pairs.0.insert(id) {
        let mut schedules = world.resource_mut::<Schedules>();
        schedules.add_systems(schedule, systems);
    }
}

pub trait AddSystemsChecked: Sized {
    /// Queues inserting systems into a schedule.
    ///
    /// This will only insert a set of systems into a given schedule once.
    fn add_systems_checked<M, S, F, C>(&mut self, schedule: S, systems: F)
    where
        S: ScheduleLabel,
        F: Fn() -> C + Send + Sync + 'static,
        C: IntoScheduleConfigs<ScheduleSystem, M> + 'static;
}

impl AddSystemsChecked for Commands<'_, '_> {
    fn add_systems_checked<M, S, F, C>(&mut self, schedule: S, systems: F)
    where
        S: ScheduleLabel,
        F: Fn() -> C + Send + Sync + 'static,
        C: IntoScheduleConfigs<ScheduleSystem, M> + 'static,
    {
        self.queue(move |world: &mut World| {
            add_systems_checked(world, schedule, systems());
        });
    }
}
