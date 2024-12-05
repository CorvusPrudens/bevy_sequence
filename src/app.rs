use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use bevy_utils::HashSet;
use std::any::TypeId;

/// `bevy_sequence`'s plugin.
pub struct SequencePlugin;

/// Sets for every `bevy_sequence` system.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SequenceSets {
    /// Evaluate node scores.
    Evaluate,

    /// Iterate over all nodes and determine which, if any, should be selected.
    Select,

    /// Emit events from the selected nodes.
    Emit,

    /// Respond to end events.
    Respond,
}

impl Plugin for SequencePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AddedSystems(Default::default()))
            .insert_resource(SelectedFragments::default())
            .add_event::<FragmentEndEvent>()
            .add_systems(
                PreUpdate,
                (
                    (
                        update_sequence_items,
                        custom_evals_ids,
                        custom_evals,
                        evaluate_limits,
                    )
                        .in_set(SequenceSets::Evaluate),
                    select_fragments.in_set(SequenceSets::Select),
                    apply_deferred
                        .after(SequenceSets::Evaluate)
                        .before(SequenceSets::Select),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    respond_to_leaf.in_set(SequenceSets::Respond),
                    clear_evals.after(SequenceSets::Respond),
                ),
            )
            .configure_sets(
                PreUpdate,
                (
                    SequenceSets::Select.after(SequenceSets::Evaluate),
                    SequenceSets::Emit.after(SequenceSets::Select),
                ),
            )
            .add_observer(sequence_begin_observer)
            .add_observer(sequence_end_observer);
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
    C: IntoSystemConfigs<M> + Send + 'static,
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
    fn add_systems_checked<M, S, C>(&mut self, schedule: S, systems: C)
    where
        S: ScheduleLabel,
        C: IntoSystemConfigs<M> + Send + 'static;
}

impl<'w, 's> AddSystemsChecked for Commands<'w, 's> {
    fn add_systems_checked<M, S, C>(&mut self, schedule: S, systems: C)
    where
        S: ScheduleLabel,
        C: IntoSystemConfigs<M> + Send + 'static,
    {
        self.queue(|world: &mut World| {
            add_systems_checked(world, schedule, systems);
        });
    }
}
