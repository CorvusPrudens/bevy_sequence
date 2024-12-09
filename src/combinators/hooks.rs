use bevy_ecs::prelude::*;
use std::marker::PhantomData;

use crate::{
    fragment::event::{BeginStage, EndStage, InsertBeginDown, InsertEndUp},
    prelude::IntoFragment,
    Threaded,
};

pub struct OnStart<T, S, M> {
    fragment: T,
    system: S,
    _marker: PhantomData<fn() -> M>,
}

pub fn on_start<T, S, M>(fragment: T, system: S) -> OnStart<T, S, M>
where
    S: IntoSystem<(), (), M> + Send + Sync + 'static,
{
    OnStart {
        fragment,
        system,
        _marker: PhantomData,
    }
}

impl<Data, Context, T, S, M> IntoFragment<Data, Context> for OnStart<T, S, M>
where
    Data: Threaded,
    S: IntoSystem<(), (), M> + Send + Sync + 'static,
    T: IntoFragment<Data, Context>,
{
    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> crate::prelude::FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        let system = commands.register_system(self.system);
        commands
            .entity(id.entity())
            .insert_begin_down(move |event, world| {
                if matches!(event.stage, BeginStage::Start) {
                    world.run_system(system).unwrap();
                }
            });

        id
    }
}

pub struct OnEnd<T, S, M> {
    fragment: T,
    system: S,
    _marker: PhantomData<fn() -> M>,
}

pub fn on_end<T, S, M>(fragment: T, system: S) -> OnEnd<T, S, M>
where
    S: IntoSystem<(), (), M> + Send + Sync + 'static,
{
    OnEnd {
        fragment,
        system,
        _marker: PhantomData,
    }
}

impl<Data, Context, T, S, M> IntoFragment<Data, Context> for OnEnd<T, S, M>
where
    Data: Threaded,
    S: IntoSystem<(), (), M> + Send + Sync + 'static,
    T: IntoFragment<Data, Context>,
{
    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> crate::prelude::FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        let system = commands.register_system(self.system);
        commands
            .entity(id.entity())
            .insert_end_up(move |event, world| {
                if matches!(event.stage, EndStage::End) {
                    world.run_system(system).unwrap();
                }
            });

        id
    }
}

pub struct OnVisit<T, S, M> {
    fragment: T,
    system: S,
    _marker: PhantomData<fn() -> M>,
}

pub fn on_visit<T, S, M>(fragment: T, system: S) -> OnVisit<T, S, M>
where
    S: IntoSystem<(), (), M> + Send + Sync + 'static,
{
    OnVisit {
        fragment,
        system,
        _marker: PhantomData,
    }
}

impl<Data, Context, T, S, M> IntoFragment<Data, Context> for OnVisit<T, S, M>
where
    Data: Threaded,
    S: IntoSystem<(), (), M> + Send + Sync + 'static,
    T: IntoFragment<Data, Context>,
{
    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> crate::prelude::FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        let system = commands.register_system(self.system);
        commands
            .entity(id.entity())
            .insert_begin_down(move |_, world| {
                world.run_system(system).unwrap();
            });

        id
    }
}
