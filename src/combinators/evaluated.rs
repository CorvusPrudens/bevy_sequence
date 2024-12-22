use crate::prelude::*;
use bevy_ecs::component::StorageType;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemId;
use std::marker::PhantomData;

pub struct EvaluatedWithId<F, T, O, M> {
    pub(super) fragment: F,
    pub(super) evaluation: T,
    pub(super) _marker: PhantomData<fn() -> (O, M)>,
}

impl<F, T, O, M> EvaluatedWithId<F, T, O, M> {
    pub fn new(fragment: F, evaluation: T) -> Self {
        Self {
            fragment,
            evaluation,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct EvalSystemId(SystemId<In<FragmentId>, Evaluation>);

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

impl<C, Data, F, T, O, M> IntoFragment<Data, C> for EvaluatedWithId<F, T, O, M>
where
    F: IntoFragment<Data, C>,
    T: IntoSystem<In<FragmentId>, O, M> + 'static,
    O: Evaluate + 'static,
    Data: Threaded,
{
    fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        let system = commands.register_system(self.evaluation.map(|input: O| input.evaluate()));

        commands.entity(id.entity()).insert(EvalSystemId(system));

        id
    }
}

pub(super) fn custom_evals_ids(
    systems: Query<(Entity, &EvalSystemId), With<Evaluation>>,
    mut commands: Commands,
) {
    let systems: Vec<_> = systems.iter().map(|(e, s)| (e, *s)).collect();

    commands.queue(|world: &mut World| {
        for (e, system) in systems {
            let evaluation = world
                .run_system_with_input(system.0, FragmentId::new(e))
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

impl<F, T, O, M> Evaluated<F, T, O, M> {
    pub fn new(fragment: F, evaluation: T) -> Self {
        Self {
            fragment,
            evaluation,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct EvalSystem(SystemId<(), Evaluation>);

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

impl<C, Data, F, T, O, M> IntoFragment<Data, C> for Evaluated<F, T, O, M>
where
    F: IntoFragment<Data, C>,
    T: IntoSystem<(), O, M> + 'static,
    O: Evaluate + 'static,
    Data: Threaded,
{
    fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        let system = commands.register_system(self.evaluation.map(|input: O| input.evaluate()));

        commands.entity(id.entity()).insert(EvalSystem(system));

        id
    }
}

pub(super) fn custom_evals(
    systems: Query<(Entity, &EvalSystem), With<Evaluation>>,
    mut commands: Commands,
) {
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
