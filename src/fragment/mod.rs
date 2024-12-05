#![allow(dead_code)]

use crate::combinators;
use crate::evaluate::{Evaluate, Evaluation};
use crate::Threaded;
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;

pub mod children;
mod leaf;

pub(crate) use leaf::respond_to_leaf;
pub use leaf::{DataLeaf, Leaf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct FragmentId(Entity);

impl FragmentId {
    pub fn new(fragment: Entity) -> Self {
        Self(fragment)
    }

    pub fn entity(&self) -> Entity {
        self.0
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
    fn limit(self, n: usize) -> combinators::Limit<Self> {
        combinators::Limit::new(self, n)
    }

    /// Set this fragment's limit to 1.
    fn once(self) -> combinators::Limit<Self> {
        self.limit(1)
    }

    /// Wrap this fragment in an evaluation.
    fn eval<S, O, M>(self, system: S) -> combinators::Evaluated<Self, S, O, M>
    where
        S: IntoSystem<(), O, M> + 'static,
        O: Evaluate + 'static,
    {
        combinators::Evaluated::new(self, system)
    }

    /// Wrap this fragment in an evaluation.
    ///
    /// This will pass the fragment's ID to the provided systme.
    fn eval_id<S, O, M>(self, system: S) -> combinators::EvaluatedWithId<Self, S, O, M>
    where
        S: IntoSystem<In<FragmentId>, O, M> + 'static,
        O: Evaluate + 'static,
    {
        combinators::EvaluatedWithId::new(self, system)
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

impl Default for EventId {
    fn default() -> Self {
        Self::new()
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
    pub fragment: FragmentId,
    pub event: EventId,
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

pub(crate) fn clear_evals(mut evals: Query<&mut Evaluation>) {
    for mut eval in evals.iter_mut() {
        *eval = Default::default();
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
