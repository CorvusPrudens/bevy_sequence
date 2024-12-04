#![allow(dead_code)]

use crate::evaluate::{Evaluate, Evaluation, FragmentState};
use crate::{fragment::Threaded, FragmentEvent, FragmentId};
use crate::{EvaluateSet, FragmentUpdate};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::utils::{all_tuples_with_size, HashSet};
use std::any::TypeId;

pub struct SequencePlugin;

impl Plugin for SequencePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AddedSystems(Default::default()))
            .add_systems(FragmentUpdate, update_sequence_items.in_set(EvaluateSet));
    }
}

pub trait IntoFragment<Context, Data: Threaded> {
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId;
}

/// An entity representing a sequence fragment.
#[derive(Debug, Default, Component)]
#[require(Evaluation, FragmentState)]
pub struct Fragment;

/// An event emitted when a leaf fragment should emit its own value.
#[derive(Debug, Event, Clone, Copy)]
pub struct FragmentEmit(FragmentId);

/// A root fragment.
#[derive(Debug, Default, Component)]
#[require(Fragment)]
pub struct Root;

/// A leaf fragment.
///
/// Leaf fragments are nodes that emit [FragmentEvent]s.
#[derive(Debug, Default, Component)]
#[require(Fragment)]
pub struct Leaf;

/// A leaf node that simply emits its contained value.
#[derive(Debug, Component)]
#[require(Leaf)]
struct DataLeaf<T>(T);

impl<T> DataLeaf<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

#[derive(Resource)]
struct AddedSystems(HashSet<TypeId>);

trait AddSystemsChecked: Sized {
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
            let id = TypeId::of::<(S, C)>();
            let mut pairs = world.get_resource_or_insert_with(|| AddedSystems(Default::default()));

            if pairs.0.insert(id) {
                let mut schedules = world.resource_mut::<Schedules>();
                schedules.add_systems(schedule, systems);
            }
        });
    }
}

impl<T, Context, Data: Threaded> IntoFragment<Context, Data> for DataLeaf<T>
where
    Data: From<T> + Clone,
{
    fn into_fragment(self, _: &Context, commands: &mut Commands) -> FragmentId {
        commands.add_systems_checked(FragmentUpdate, watch_leaves::<Data>);

        let entity = commands.spawn(DataLeaf::<Data>(self.0.into()));

        FragmentId::new(entity.id())
    }
}

fn watch_leaves<Data>(
    leaves: Query<&DataLeaf<Data>>,
    mut reader: EventReader<FragmentEmit>,
    mut writer: EventWriter<FragmentEvent<Data>>,
) where
    Data: Threaded + Clone,
{
    for FragmentEmit(fragment) in reader.read() {
        if let Ok(leaf) = leaves.get(fragment.0) {
            writer.send(FragmentEvent {
                id: *fragment,
                data: leaf.0.clone(),
            });
        }
    }
}

#[derive(Component)]
#[require(Fragment)]
struct Sequence;

pub struct SequenceContainer<F> {
    fragments: F,
    id: FragmentId,
}

fn update_sequence_items(
    q: Query<(&Children, &FragmentState), With<Sequence>>,
    mut children: Query<(&mut Evaluation, &FragmentState)>,
) {
    for (seq, outer_state) in q.iter() {
        // look for the first item that has finished equal to the container
        let mut first_selected = false;
        for child in seq.iter() {
            let Ok((mut eval, state)) = children.get_mut(*child) else {
                continue;
            };

            if !first_selected && state.completed <= outer_state.completed {
                first_selected = true;
                eval.merge(true.evaluate());

                continue;
            }

            eval.merge(false.evaluate());
        }
    }
}

macro_rules! seq_frag {
    ($count:literal, $($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<Context, Data, $($ty),*> IntoFragment<Context, Data> for ($($ty,)*)
        where
            Data: Threaded,
            Context: Threaded,
            $($ty: IntoFragment<Context, Data>),*
        {
            #[allow(unused)]
            fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
                let ($($ty,)*) = self;

                let children: [_; $count] = [
                    $($ty.into_fragment(context, commands).0),*
                ];

                let mut entity = commands.spawn_empty();
                FragmentId::new(
                    entity
                        .add_children(&children)
                        .insert(Sequence)
                        .id()
                )
            }
        }
    };
}

all_tuples_with_size!(seq_frag, 0, 15, T);

fn test() -> impl IntoFragment<(), String> {
    ("Hello, world!", "How are you?")
}

impl IntoFragment<(), String> for &'static str {
    fn into_fragment(self, context: &(), commands: &mut Commands) -> FragmentId {
        <_ as IntoFragment<(), String>>::into_fragment(DataLeaf::new(self), context, commands)
    }
}

fn descend_tree(
    node: Entity,
    evaluation: Evaluation,
    fragments: &Query<(&Evaluation, Option<&Children>, Option<&Leaf>)>,
    leaves: &mut Vec<(Entity, Evaluation)>,
) {
    let Ok((eval, children, leaf)) = fragments.get(node) else {
        return;
    };

    let new_eval = *eval & evaluation;

    if new_eval.result.unwrap_or_default() {
        if leaf.is_some() {
            leaves.push((node, new_eval));
        } else {
            for child in children.iter().flat_map(|c| c.iter()) {
                descend_tree(*child, new_eval, fragments, leaves);
            }
        }
    }
}

fn evaluate_fragments<Data>(
    roots: Query<(Entity, &Evaluation), With<Root>>,
    fragments: Query<(&Evaluation, Option<&Children>, Option<&Leaf>)>,
    leaf_data: Query<&DataLeaf<Data>>,
    mut writer: EventWriter<FragmentEvent<Data>>,
    mut commands: Commands,
) where
    Data: Threaded + Clone,
{
    // traverse trees to build up full evaluatinos
    let mut leaves = Vec::new();

    for (root, eval) in roots.iter() {
        descend_tree(root, *eval, &fragments, &mut leaves);
    }

    leaves.sort_by_key(|(_, e)| e.count);

    if let Some((_, eval)) = leaves.first() {
        let selections = leaves.iter().take_while(|e| e.1.count == eval.count);

        for (fragment, _) in selections {
            if let Ok(leaf) = leaf_data.get(*fragment) {
                writer.send(FragmentEvent {
                    id: FragmentId(*fragment),
                    data: leaf.0.clone(),
                });
            }
        }
    }
}
