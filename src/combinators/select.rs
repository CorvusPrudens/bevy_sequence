use crate::fragment::children::IntoChildren;
use crate::prelude::*;
use bevy_ecs::component::StorageType;
use bevy_ecs::prelude::*;
use bevy_ecs::system::{IntoSystem, SystemId};
use bevy_hierarchy::prelude::*;
use std::marker::PhantomData;

/// A combinator that selects exactly one fragment from a tuple based on a system's output.
///
/// The system should return a `usize` representing the chosen child's index.
/// If the returned index is out of range, no child will be selected.
pub struct SelectFragment<F, T, M> {
    fragments: F,
    system: T,
    _marker: PhantomData<fn() -> M>,
}

pub fn select<F, T, M>(fragments: F, system: T) -> SelectFragment<F, T, M>
where
    T: IntoSystem<(), usize, M> + 'static,
{
    SelectFragment {
        fragments,
        system,
        _marker: PhantomData,
    }
}

/// A component inserted by the `Select` combinator to track the chosen index system.
#[derive(Clone, Copy)]
pub struct SelectSystem(SystemId<(), usize>);

#[derive(Clone, Copy, Component)]
pub(super) struct SelectActiveNode(usize);

// Here we automatically clean up the system when this component is removed.
impl Component for SelectSystem {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy_ecs::component::ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let eval = world.get::<SelectSystem>(entity).unwrap().0;
            world.commands().unregister_system(eval);
        });
    }

    fn register_required_components(
        component_id: bevy_ecs::component::ComponentId,
        components: &mut bevy_ecs::component::Components,
        storages: &mut bevy_ecs::storage::Storages,
        required_components: &mut bevy_ecs::component::RequiredComponents,
        inheritance_depth: u16,
    ) {
        <Fragment as bevy_ecs::component::Component>::register_required_components(
            component_id,
            components,
            storages,
            required_components,
            inheritance_depth + 1,
        );
    }
}

impl<Context, Data, F, T, M> IntoFragment<Data, Context> for SelectFragment<F, T, M>
where
    Data: Threaded,
    F: IntoChildren<Data, Context>,
    T: IntoSystem<(), usize, M> + 'static,
{
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
        let children = self.fragments.into_children(context, commands);

        // Register the provided system.
        let system_id = commands.register_system(self.system);

        // Spawn a parent entity with Select and SelectSystem
        let parent = commands
            .spawn((SelectSystem(system_id), SelectActiveNode(0)))
            .add_children(children.as_ref())
            .id();

        FragmentId::new(parent)
    }
}

// This isn't _quite_ right since we'll need to handle
// continuation and such once an item is actually chosen.
pub(super) fn update_select_items(
    choices: Query<(
        Entity,
        &Children,
        &FragmentState,
        &SelectSystem,
        &SelectActiveNode,
    )>,
    mut commands: Commands,
) {
    let choices: Vec<_> = choices
        .iter()
        .map(|(e, c, state, s, active)| {
            (e, c.to_vec(), state.active_events.is_empty(), *s, *active)
        })
        .collect();

    commands.queue(|world: &mut World| {
        for (e, children, empty, choice_system, active) in choices {
            let result = if empty {
                let result = world.run_system(choice_system.0).unwrap();
                world.entity_mut(e).insert(SelectActiveNode(result));
                result
            } else {
                active.0
            };

            for (i, child) in children.iter().enumerate() {
                let mut child = world.entity_mut(*child);
                if let Some(mut evaluation) = child.get_mut::<Evaluation>() {
                    evaluation.merge((result == i).evaluate());
                }
            }
        }
    });
}
