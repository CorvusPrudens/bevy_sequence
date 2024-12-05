use crate::fragment::children::IntoChildren;
use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::system::{IntoSystem, SystemId};
use bevy_hierarchy::prelude::*;
use std::marker::PhantomData;

/// A combinator that selects exactly one fragment from a tuple based on a system's output.
///
/// The system should return a `usize` representing the chosen child's index.
/// If the returned index is out of range, no child will be selected.
pub struct ChoiceFragment<F, T, O, M> {
    fragments: F,
    system: T,
    _marker: PhantomData<fn() -> (O, M)>,
}

impl<F, T, O, M> ChoiceFragment<F, T, O, M> {
    pub fn new(fragments: F, system: T) -> Self {
        ChoiceFragment {
            fragments,
            system,
            _marker: PhantomData,
        }
    }
}

/// A component inserted by the `Choice` combinator to track the chosen index system.
#[derive(Component)]
pub struct ChoiceSystem {
    system_id: SystemId<(), usize>,
}

impl<Context, Data, F, T, O, M> IntoFragment<Context, Data> for ChoiceFragment<F, T, O, M>
where
    Data: Threaded,
    F: IntoChildren<Context, Data>,
    T: IntoSystem<(), O, M> + 'static,
    O: Into<usize> + 'static,
{
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
        let children = self.fragments.into_children(context, commands);

        // Register the provided system.
        let system_id = commands.register_system(self.system.map(|idx: O| idx.into()));

        // Spawn a parent entity with Choice and ChoiceSystem.
        let parent = commands
            .spawn((
                Fragment,
                Evaluation::default(),
                FragmentState::default(),
                Choice,
                ChoiceSystem { system_id },
            ))
            .add_children(children.as_ref())
            .id();

        FragmentId::new(parent)
    }
}

/// A marker component for Choice parent entities.
#[derive(Component)]
#[require(Fragment)]
pub struct Choice;

pub(super) fn update_choice_items(
    choices: Query<(&Children, &ChoiceSystem, &FragmentState), With<Choice>>,
    mut children_query: Query<&mut Evaluation>,
    mut commands: Commands,
) {
    // For each Choice entity, run the system and select one child
    for (children, choice_system, state) in choices.iter() {
        // If the fragment is inactive (no active events), we can re-run the system to select a child.
        // Otherwise, if it's active (an event triggered), the chosen child won't be re-selected until completion.

        let inactive = state.active_events.is_empty();

        // If the fragment is not ready for re-selection, just continue.
        if !inactive {
            // Mark all children as false if needed.
            for child in children.iter() {
                if let Ok(mut eval) = children_query.get_mut(*child) {
                    eval.merge(false.evaluate());
                }
            }
            continue;
        }

        // Run the choice system to get the selected index.
        let selected_index = commands
            .queue(|world: &mut World| world.run_system(choice_system.system_id))
            .unwrap();

        // For each child, if it's the chosen one, set evaluation = true, else false.
        for (i, child) in children.iter().enumerate() {
            if let Ok(mut eval) = children_query.get_mut(*child) {
                if i == selected_index {
                    eval.merge(true.evaluate());
                } else {
                    eval.merge(false.evaluate());
                }
            }
        }
    }
}
