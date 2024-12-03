use super::{FragmentNode, IntoFragment, Threaded};
use crate::{FragmentEvent, FragmentUpdate};
use bevy::{ecs::event::EventRegistry, prelude::*};
use std::marker::PhantomData;

/// Maps a fragment event.
pub struct Mapped<F, S, E, Data> {
    pub(super) fragment: F,
    pub(super) event: S,
    pub(super) _marker: PhantomData<fn() -> (S, E, Data)>,
}

impl<Context, Data, F, S, E> IntoFragment<Context, Data> for Mapped<F, S, E, Data>
where
    F: IntoFragment<Context, Data>,
    S: FnMut(&FragmentEvent<Data>) -> E + Send + Sync + 'static,
    E: Event + Clone,
    Data: Threaded,
{
    type Fragment = F::Fragment;

    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> (Self::Fragment, FragmentNode) {
        let (fragment, node) = self.fragment.into_fragment(context, commands);

        let leaves = node.leaves();
        let mut map = self.event;
        commands.queue(move |world: &mut World| {
            if !world.contains_resource::<Events<E>>() {
                EventRegistry::register_event::<E>(world);
            }

            let mut schedules = world.resource_mut::<Schedules>();
            schedules.add_systems(
                FragmentUpdate,
                move |mut raw_events: EventReader<FragmentEvent<Data>>,
                      mut wrapped: EventWriter<E>| {
                    for event in raw_events.read() {
                        if leaves.contains(&event.id) {
                            wrapped.send(map(event));
                        }
                    }
                },
            );
        });

        (fragment, node)
    }
}
