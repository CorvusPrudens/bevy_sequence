use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::Children;
use bevy_log::prelude::*;
use bevy_utils::hashbrown::hash_map::Entry;
use bevy_utils::HashMap;
use std::iter::zip;
use std::{any::TypeId, borrow::Cow};

/// Save a tree with a given name.
pub struct Save<T> {
    fragment: T,
    name: Cow<'static, str>,
}

impl<T: 'static> Save<T> {
    pub fn new(fragment: T, name: Cow<'static, str>) -> Self {
        Self { fragment, name }
    }
}

#[derive(Debug, Clone, Resource, Default)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedSequences(HashMap<Cow<'static, str>, SavedSequence>);

#[derive(Debug, Clone, Resource)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedSequence {
    nodes: SavedNode,
    #[cfg(debug_assertions)]
    #[cfg_attr(feature = "serde", serde(skip))]
    ty: Option<TypeId>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "reflect", reflect(no_field_bounds))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedNode {
    state: FragmentState,
    children: Vec<SavedNode>,
}

#[derive(Debug, Component, Clone)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SequenceState {
    name: Cow<'static, str>,
    nodes: Option<SavedNode>,
    #[cfg(debug_assertions)]
    #[cfg_attr(feature = "serde", serde(skip))]
    ty: Option<TypeId>,
}

impl<T, C, D> IntoFragment<D, C> for Save<T>
where
    T: IntoFragment<D, C> + 'static,
    D: Threaded,
{
    fn into_fragment(self, context: &C, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        commands.entity(id.entity()).insert(SequenceState {
            name: self.name,
            nodes: None,
            #[cfg(debug_assertions)]
            ty: Some(TypeId::of::<T>()),
        });

        id
    }
}

fn apply_saved_state(
    name: &str,
    node: Entity,
    state: &SavedNode,
    nodes: &mut Query<&mut FragmentState, With<Fragment>>,
    children_query: &Query<&Children>,
) -> Option<()> {
    let mut frag_state = nodes.get_mut(node).ok()?;
    let children = children_query.get(node).ok();

    *frag_state = state.state.clone();

    match children {
        Some(children) if children.len() != state.children.len() => {
            warn!(
                "mismatch between saved state and entities for sequence \"{name}\": saved children {} does not match entities: {}",
                state.children.len(), children.len(),
            );
        }
        None if !state.children.is_empty() => {
            warn!(
                "mismatch between saved state and entities for sequence \"{name}\": expected children, but entity has none"
            );
        }
        Some(children) => {
            for (child, child_state) in zip(children, &state.children) {
                apply_saved_state(name, *child, child_state, nodes, children_query);
            }
        }
        _ => {}
    }

    Some(())
}

pub(super) fn load_sequence(
    trigger: Trigger<OnAdd, SequenceState>,
    mut sequence: Query<&mut SequenceState>,
    mut nodes: Query<&mut FragmentState, With<Fragment>>,
    children: Query<&Children>,
    saved: Res<SavedSequences>,
) {
    let source = trigger.entity();

    let Ok(mut sequence) = sequence.get_mut(source) else {
        return;
    };

    if sequence.nodes.is_none() {
        sequence.nodes = saved.0.get(&sequence.name).map(|s| s.nodes.clone());
    }

    if let Some(saved_nodes) = &sequence.nodes {
        apply_saved_state(&sequence.name, source, saved_nodes, &mut nodes, &children);
    }
}

fn get_saved_state(
    node: Entity,
    state: &mut SavedNode,
    nodes: &Query<(&FragmentState, Option<&Children>), With<Fragment>>,
) -> Option<()> {
    let (node_state, children) = nodes.get(node).ok()?;

    state.state = node_state.clone();

    if let Some(children) = children {
        state.children.resize(children.len(), Default::default());

        for (child, child_state) in zip(children, &mut state.children) {
            get_saved_state(*child, child_state, nodes);
        }
    }

    Some(())
}

pub(super) fn sync_sequence(
    mut sequences: Query<(Entity, &mut SequenceState)>,
    nodes: Query<(&FragmentState, Option<&Children>), With<Fragment>>,
    mut saved: ResMut<SavedSequences>,
) {
    for (root, mut sequence) in sequences.iter_mut() {
        let state = sequence.nodes.get_or_insert(Default::default());

        get_saved_state(root, state, &nodes);

        let entry = saved.0.entry(sequence.name.clone());
        match entry {
            Entry::Occupied(mut occ) => {
                let occ = occ.get_mut();
                #[cfg(debug_assertions)]
                {
                    if matches!((occ.ty, sequence.ty), (Some(a), Some(b)) if a != b) {
                        warn_once!(
                            "name \"{}\" used for multiple distinct sequences",
                            sequence.name
                        );
                    }
                    occ.ty = sequence.ty;
                }
                occ.nodes = sequence.nodes.clone().unwrap_or_default();
            }
            Entry::Vacant(v) => {
                v.insert(SavedSequence {
                    nodes: sequence.nodes.clone().unwrap_or_default(),
                    #[cfg(debug_assertions)]
                    ty: sequence.ty,
                });
            }
        }
    }
}
