use crate::combinators;
use crate::evaluate::{Evaluate, Evaluation};
use crate::Threaded;
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;

pub mod children;
pub mod event;
mod leaf;

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

pub trait IntoFragment<Data: Threaded, Context = ()> {
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId;
}

pub fn spawn_root<Data: Threaded>(fragment: impl IntoFragment<Data>, commands: &mut Commands) {
    let root = fragment.into_fragment(&(), commands);
    commands.entity(root.0).insert(Root);
}

pub fn spawn_root_with_context<Data: Threaded, Context: Component>(
    fragment: impl IntoFragment<Data, Context>,
    context: Context,
    commands: &mut Commands,
) {
    let root = fragment.into_fragment(&context, commands);
    commands.entity(root.0).insert((Root, context));
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

#[derive(Debug, Component, Default, Clone, PartialEq, Eq)]
pub struct FragmentState {
    pub triggered: usize,
    pub completed: usize,
    pub active_events: event::ActiveEvents,
}

/// An entity representing a sequence fragment.
#[derive(Debug, Default, Component)]
#[require(Evaluation, FragmentState, event::EventPath)]
pub struct Fragment;

/// A root fragment.
#[derive(Debug, Default, Component, Clone)]
#[require(Fragment)]
pub struct Root;

pub(crate) fn clear_evals(mut evals: Query<&mut Evaluation>) {
    for mut eval in evals.iter_mut() {
        *eval = Default::default();
    }
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
