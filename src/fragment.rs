#![allow(dead_code)]

use crate::app::SequenceSets;
use crate::evaluate::{Evaluate, Evaluation};
use crate::Threaded;
use bevy_app::prelude::*;
use bevy_ecs::component::StorageType;
use bevy_ecs::event::EventRegistry;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{ScheduleLabel, SystemSet};
use bevy_ecs::system::SystemId;
use bevy_hierarchy::prelude::*;
use bevy_utils::{all_tuples_with_size, HashSet};
use std::any::TypeId;
use std::marker::PhantomData;

mod leaf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct FragmentId(Entity);

impl FragmentId {
    pub fn new(fragment: Entity) -> Self {
        Self(fragment)
    }
}

pub trait IntoFragment<Context, Data: Threaded> {
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId;
}

pub fn spawn_root<Context: Component, Data: Threaded>(
    fragment: impl IntoFragment<Context, Data>,
    context: Context,
    commands: &mut Commands,
) {
    let root = fragment.into_fragment(&context, commands);
    commands.entity(root.0).insert((context, Root));
}

impl<T> FragmentExt for T {}

pub trait FragmentExt: Sized {
    /// Limit this fragment to `n` triggers.
    fn limit(self, n: usize) -> Limit<Self> {
        Limit::new(self, n)
    }

    /// Set this fragment's limit to 1.
    fn once(self) -> Limit<Self> {
        self.limit(1)
    }

    /// Wrap this fragment in an evaluation.
    fn eval<S, O, M>(self, system: S) -> Evaluated<Self, S, O, M>
    where
        S: IntoSystem<(), O, M> + 'static,
        O: Evaluate + 'static,
    {
        Evaluated {
            fragment: self,
            evaluation: system,
            _marker: PhantomData,
        }
    }

    /// Wrap this fragment in an evaluation.
    ///
    /// This will pass the fragment's ID to the provided systme.
    fn eval_id<S, O, M>(self, system: S) -> EvaluatedWithId<Self, S, O, M>
    where
        S: IntoSystem<In<FragmentId>, O, M> + 'static,
        O: Evaluate + 'static,
    {
        EvaluatedWithId {
            fragment: self,
            evaluation: system,
            _marker: PhantomData,
        }
    }
}

/// A unique ID generated for every emitted event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(u64);

impl EventId {
    pub fn new() -> Self {
        use rand::prelude::*;

        Self(rand::thread_rng().gen())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ActiveEvents(Vec<EventId>);

impl core::ops::Deref for ActiveEvents {
    type Target = [EventId];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ActiveEvents {
    pub fn new(events: Vec<EventId>) -> Self {
        Self(events)
    }

    /// Remove an ID by value.
    ///
    /// If the ID existed in the set and was removed,
    /// this returns true.
    pub fn remove(&mut self, id: EventId) -> bool {
        if let Some(index) = self.iter().position(|e| *e == id) {
            self.0.swap_remove(index);
            true
        } else {
            false
        }
    }

    pub fn insert(&mut self, id: EventId) {
        self.0.push(id);
    }
}

#[derive(Debug, Component, Default, Clone, PartialEq, Eq)]
pub struct FragmentState {
    pub triggered: usize,
    pub completed: usize,
    pub active_events: ActiveEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdPair {
    fragment: FragmentId,
    event: EventId,
}

#[derive(Debug, Event, Clone)]
pub struct FragmentEvent<Data> {
    pub id: IdPair,
    pub data: Data,
}

impl<Data> FragmentEvent<Data> {
    pub fn end(&self) -> FragmentEndEvent {
        FragmentEndEvent(self.id)
    }
}

#[derive(Debug, Event, Clone, Copy)]
pub struct FragmentEndEvent(IdPair);

/// An entity representing a sequence fragment.
#[derive(Debug, Default, Component)]
#[require(Evaluation, FragmentState)]
pub struct Fragment;

/// A root fragment.
#[derive(Debug, Default, Component)]
#[require(Fragment)]
pub struct Root;

fn clear_evals(mut evals: Query<&mut Evaluation>) {
    for mut eval in evals.iter_mut() {
        *eval = Default::default();
    }
}

#[derive(Component)]
#[require(Fragment)]
struct Sequence;

fn update_sequence_items(
    q: Query<(&Children, &FragmentState), With<Sequence>>,
    mut children: Query<(&mut Evaluation, &FragmentState)>,
) {
    for (seq, outer_state) in q.iter() {
        let inactive = outer_state.active_events.is_empty();

        // look for the first item that has finished equal to the container
        let mut first_selected = false;
        for child in seq.iter() {
            let Ok((mut eval, state)) = children.get_mut(*child) else {
                continue;
            };

            if inactive
                && !first_selected
                && state.active_events.is_empty()
                && state.completed <= outer_state.completed
            {
                first_selected = true;
                eval.merge(true.evaluate());

                continue;
            }

            eval.merge(false.evaluate());
        }
    }
}

#[derive(Debug, Event)]
pub struct BeginEvent {
    pub id: IdPair,
    pub kind: BeginKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BeginKind {
    Start,
    Visit,
}

#[derive(Debug, Event)]
pub struct EndEvent {
    pub id: IdPair,
    pub kind: EndKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EndKind {
    End,
    Visit,
}

fn sequence_begin_observer(
    trigger: Trigger<BeginEvent>,
    mut parent: Query<(&mut FragmentState, &Children), With<Sequence>>,
    child: Query<&Parent>,
    mut commands: Commands,
) {
    let child_id = trigger.entity();
    let Ok(parent_id) = child.get(child_id).map(|p| p.get()) else {
        return;
    };

    let Ok((mut state, children)) = parent.get_mut(parent_id) else {
        return;
    };

    let first = children.first().is_some_and(|f| *f == child_id);
    state.active_events.insert(trigger.id.event);

    let kind = if first && trigger.kind == BeginKind::Start {
        state.triggered += 1;
        BeginKind::Start
    } else {
        BeginKind::Visit
    };

    commands.trigger_targets(
        BeginEvent {
            id: trigger.id,
            kind,
        },
        parent_id,
    );

    // info!("observed begin event! {trigger:?}");
}

fn sequence_end_observer(
    trigger: Trigger<EndEvent>,
    mut parent: Query<(&mut FragmentState, &Children), With<Sequence>>,
    child: Query<&Parent>,
    mut commands: Commands,
) {
    let child_id = trigger.entity();
    let Ok(parent_id) = child.get(child_id).map(|p| p.get()) else {
        return;
    };

    let Ok((mut state, children)) = parent.get_mut(parent_id) else {
        return;
    };

    let last = children.last().is_some_and(|f| *f == child_id);

    if state.active_events.remove(trigger.id.event) {
        let kind = if last && trigger.kind == EndKind::End {
            state.completed += 1;
            EndKind::End
        } else {
            EndKind::Visit
        };

        commands.trigger_targets(
            EndEvent {
                id: trigger.id,
                kind,
            },
            parent_id,
        );
    }

    // info!("observed end event! {trigger:?}");
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

                FragmentId::new(commands.spawn(Sequence).add_children(&children).id())
            }
        }
    };
}

all_tuples_with_size!(seq_frag, 0, 15, T);

/// Recursively walk the tree depth-first, building
/// up evaluations we go.
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

#[derive(Debug, Default, Resource)]
pub struct SelectedFragments(pub Vec<Entity>);

pub fn select_fragments(
    roots: Query<(Entity, &Evaluation), With<Root>>,
    fragments: Query<(&Evaluation, Option<&Children>, Option<&Leaf>)>,
    f: Query<(&Evaluation, &FragmentState)>,
    mut selected_fragments: ResMut<SelectedFragments>,
) {
    // traverse trees to build up full evaluatinos
    let mut leaves = Vec::new();

    for (root, eval) in roots.iter() {
        descend_tree(root, *eval, &fragments, &mut leaves);
    }

    leaves.sort_by_key(|(_, e)| e.count);

    selected_fragments.0.clear();

    if let Some((_, eval)) = leaves.first() {
        selected_fragments.0.extend(
            leaves
                .iter()
                .take_while(|e| e.1.count == eval.count)
                .map(|(e, _)| *e),
        );
    }
}

/// A wrapper fragment that limits its children to a certain number of executions.
pub struct Limit<T> {
    fragment: T,
    limit: usize,
}

impl<T> Limit<T> {
    pub fn new(fragment: T, limit: usize) -> Self {
        Self { fragment, limit }
    }
}

#[derive(Debug, Component)]
pub struct LimitItem(usize);

impl<T, C, D> IntoFragment<C, D> for Limit<T>
where
    T: IntoFragment<C, D>,
    D: Threaded,
{
    fn into_fragment(self, context: &C, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        commands.entity(id.0).insert(LimitItem(self.limit));

        id
    }
}

fn evaluate_limits(mut fragments: Query<(&mut Evaluation, &FragmentState, &LimitItem)>) {
    for (mut eval, state, limit) in fragments.iter_mut() {
        if state.completed >= limit.0 {
            eval.merge(false.evaluate());
        }
    }
}

pub struct EvaluatedWithId<F, T, O, M> {
    pub(super) fragment: F,
    pub(super) evaluation: T,
    pub(super) _marker: PhantomData<fn() -> (O, M)>,
}

#[derive(Clone, Copy)]
struct EvalSystemId(SystemId<In<FragmentId>, Evaluation>);

// Here we automatically clean up the system when this component is removed.
impl Component for EvalSystemId {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy_ecs::component::ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let eval = world.get::<EvalSystemId>(entity).unwrap().0;
            world.commands().unregister_system(eval);
        });
    }
}

impl<Context, Data, F, T, O, M> IntoFragment<Context, Data> for EvaluatedWithId<F, T, O, M>
where
    F: IntoFragment<Context, Data>,
    T: IntoSystem<In<FragmentId>, O, M> + 'static,
    O: Evaluate + 'static,
    Data: Threaded,
{
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        let system = commands.register_system(self.evaluation.map(|input: O| input.evaluate()));

        commands.entity(id.0).insert(EvalSystemId(system));

        id
    }
}

fn custom_evals_ids(
    systems: Query<(Entity, &EvalSystemId), With<Evaluation>>,
    mut commands: Commands,
) {
    let systems: Vec<_> = systems.iter().map(|(e, s)| (e, *s)).collect();

    commands.queue(|world: &mut World| {
        for (e, system) in systems {
            let evaluation = world
                .run_system_with_input(system.0, FragmentId(e))
                .unwrap();
            let mut entity_eval = world.entity_mut(e);
            let mut entity_eval = entity_eval.get_mut::<Evaluation>().unwrap();
            entity_eval.merge(evaluation);
        }
    });
}

pub struct Evaluated<F, T, O, M> {
    pub(super) fragment: F,
    pub(super) evaluation: T,
    pub(super) _marker: PhantomData<fn() -> (O, M)>,
}

#[derive(Clone, Copy)]
struct EvalSystem(SystemId<(), Evaluation>);

// Here we automatically clean up the system when this component is removed.
impl Component for EvalSystem {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy_ecs::component::ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let eval = world.get::<EvalSystemId>(entity).unwrap().0;
            world.commands().unregister_system(eval);
        });
    }
}

impl<Context, Data, F, T, O, M> IntoFragment<Context, Data> for Evaluated<F, T, O, M>
where
    F: IntoFragment<Context, Data>,
    T: IntoSystem<(), O, M> + 'static,
    O: Evaluate + 'static,
    Data: Threaded,
{
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        let system = commands.register_system(self.evaluation.map(|input: O| input.evaluate()));

        commands.entity(id.0).insert(EvalSystem(system));

        id
    }
}

fn custom_evals(systems: Query<(Entity, &EvalSystem), With<Evaluation>>, mut commands: Commands) {
    let systems: Vec<_> = systems.iter().map(|(e, s)| (e, *s)).collect();

    commands.queue(|world: &mut World| {
        for (e, system) in systems {
            let evaluation = world.run_system(system.0).unwrap();
            let mut entity_eval = world.entity_mut(e);
            let mut entity_eval = entity_eval.get_mut::<Evaluation>().unwrap();
            entity_eval.merge(evaluation);
        }
    });
}
