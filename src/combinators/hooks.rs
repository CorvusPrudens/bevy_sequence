use crate::fragment::Context;
use bevy_ecs::prelude::*;
use std::marker::PhantomData;

use crate::{
    fragment::event::{BeginStage, EndStage, InsertBeginDown, InsertEndUp},
    prelude::IntoFragment,
    Threaded,
};

trait AsInput<In: SystemInput> {
    fn as_input(&self, func: impl FnOnce(In::Inner<'_>));
}

impl<T> AsInput<()> for Context<T> {
    fn as_input(&self, func: impl FnOnce(())) {
        func(())
    }
}

impl<T: 'static> AsInput<InRef<'_, T>> for Context<T> {
    fn as_input(&self, func: impl FnOnce(&T)) {
        let read = self.read().unwrap();

        func(&read)
    }
}

impl<T: 'static> AsInput<InMut<'_, T>> for Context<T> {
    fn as_input(&self, func: impl FnOnce(&mut T)) {
        let mut read = self.write().unwrap();

        func(&mut read)
    }
}

pub struct OnStart<T, S, In, M> {
    fragment: T,
    system: S,
    _marker: PhantomData<fn(In) -> M>,
}

pub fn on_start<T, S, In, M>(fragment: T, system: S) -> OnStart<T, S, In, M>
where
    S: IntoSystem<In, (), M> + Send + Sync + 'static,
    In: bevy_ecs::system::SystemInput,
{
    OnStart {
        fragment,
        system,
        _marker: PhantomData,
    }
}

impl<Data, C, T, S, In, M> IntoFragment<Data, C> for OnStart<T, S, In, M>
where
    Data: Threaded,
    S: IntoSystem<In, (), M> + Send + Sync + 'static,
    T: IntoFragment<Data, C>,
    In: bevy_ecs::system::SystemInput,
    Context<C>: AsInput<In>,
    C: Send + Sync + 'static,
{
    fn into_fragment(
        self,
        context: &Context<C>,
        commands: &mut Commands,
    ) -> crate::prelude::FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        commands.entity(id.entity()).insert_begin_down({
            let context = context.clone();
            let mut system = IntoSystem::<In, (), M>::into_system(self.system);

            move |event, world| {
                if matches!(event.stage, BeginStage::Start) {
                    <_ as AsInput<In>>::as_input(&context, |input| {
                        // do we need to do this every time?
                        system.initialize(world);
                        system.run(input, world);
                    });

                    world.flush();
                }
            }
        });

        id
    }
}

pub struct OnEnd<T, S, In, M> {
    fragment: T,
    system: S,
    _marker: PhantomData<fn(In) -> M>,
}

pub fn on_end<T, S, In, M>(fragment: T, system: S) -> OnEnd<T, S, In, M>
where
    S: IntoSystem<In, (), M> + Send + Sync + 'static,
    In: bevy_ecs::system::SystemInput,
{
    OnEnd {
        fragment,
        system,
        _marker: PhantomData,
    }
}

impl<Data, C, T, S, In, M> IntoFragment<Data, C> for OnEnd<T, S, In, M>
where
    Data: Threaded,
    S: IntoSystem<In, (), M> + Send + Sync + 'static,
    T: IntoFragment<Data, C>,
    In: bevy_ecs::system::SystemInput,
    Context<C>: AsInput<In>,
    C: Send + Sync + 'static,
{
    fn into_fragment(
        self,
        context: &Context<C>,
        commands: &mut Commands,
    ) -> crate::prelude::FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        commands.entity(id.entity()).insert_end_up({
            let context = context.clone();
            let mut system = IntoSystem::<In, (), M>::into_system(self.system);

            move |event, world| {
                if matches!(event.stage, EndStage::End) {
                    <_ as AsInput<In>>::as_input(&context, |input| {
                        // do we need to do this every time?
                        system.initialize(world);
                        system.run(input, world);
                    });

                    world.flush();
                }
            }
        });

        id
    }
}

pub struct OnVisit<T, S, In, M> {
    fragment: T,
    system: S,
    _marker: PhantomData<fn(In) -> M>,
}

pub fn on_visit<T, S, In, M>(fragment: T, system: S) -> OnVisit<T, S, In, M>
where
    S: IntoSystem<In, (), M> + Send + Sync + 'static,
    In: bevy_ecs::system::SystemInput,
{
    OnVisit {
        fragment,
        system,
        _marker: PhantomData,
    }
}

impl<Data, C, T, S, In, M> IntoFragment<Data, C> for OnVisit<T, S, In, M>
where
    Data: Threaded,
    S: IntoSystem<In, (), M> + Send + Sync + 'static,
    T: IntoFragment<Data, C>,
    In: bevy_ecs::system::SystemInput,
    Context<C>: AsInput<In>,
    C: Send + Sync + 'static,
{
    fn into_fragment(
        self,
        context: &Context<C>,
        commands: &mut Commands,
    ) -> crate::prelude::FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        commands.entity(id.entity()).insert_begin_down({
            let context = context.clone();
            let mut system = IntoSystem::<In, (), M>::into_system(self.system);

            move |_, world| {
                <_ as AsInput<In>>::as_input(&context, |input| {
                    // do we need to do this every time?
                    system.initialize(world);
                    system.run(input, world);
                });

                world.flush();
            }
        });

        id
    }
}
