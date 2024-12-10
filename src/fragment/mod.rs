use crate::combinators::or::OrItem;
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

#[derive(Debug, Component, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FragmentState {
    pub triggered: usize,
    pub completed: usize,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub active: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
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
    fragments: &Query<(
        &Evaluation,
        Option<&Children>,
        Option<&Leaf>,
        Option<&OrItem>,
    )>,
    leaves: &mut Vec<(Entity, Evaluation)>,
    first_eval: &mut Option<Evaluation>,
) {
    let Ok((eval, children, leaf, or)) = fragments.get(node) else {
        return;
    };

    let eval = match (*first_eval, or) {
        (Some(first), Some(_)) if first.result.is_some() => {
            *eval & (!first.result.unwrap_or_default()).evaluate()
        }
        _ => *eval,
    };

    let new_eval = eval & evaluation;

    if first_eval.is_none() {
        *first_eval = Some(new_eval);
    }

    if new_eval.result.unwrap_or_default() {
        if leaf.is_some() {
            leaves.push((node, new_eval));
        } else {
            let mut first_eval = None;
            for child in children.iter().flat_map(|c| c.iter()) {
                descend_tree(*child, new_eval, fragments, leaves, &mut first_eval);
            }
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct SelectedFragments(pub Vec<Entity>);

pub fn select_fragments(
    roots: Query<(Entity, &Evaluation), With<Root>>,
    fragments: Query<(
        &Evaluation,
        Option<&Children>,
        Option<&Leaf>,
        Option<&OrItem>,
    )>,
    mut selected_fragments: ResMut<SelectedFragments>,
) {
    // traverse trees to build up full evaluatinos
    let mut leaves = Vec::new();

    for (root, eval) in roots.iter() {
        let mut or = None;
        descend_tree(root, *eval, &fragments, &mut leaves, &mut or);
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
