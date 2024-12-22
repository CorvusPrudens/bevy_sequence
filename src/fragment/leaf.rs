use super::event::InsertBeginDown;
use super::Context;
use crate::prelude::*;
use bevy_ecs::event::EventRegistry;
use bevy_ecs::prelude::*;

/// A leaf fragment.
///
/// Leaf fragments are nodes that emit [FragmentEvent]s.
#[derive(Debug, Default, Component)]
#[require(Fragment)]
pub struct Leaf;

/// A leaf node that simply emits its contained value.
#[derive(Debug, Component)]
#[require(Leaf)]
pub struct DataLeaf<T>(T);

impl<T> DataLeaf<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T, Data: Threaded, C> IntoFragment<Data, C> for DataLeaf<T>
where
    Data: From<T> + Clone,
{
    fn into_fragment(self, _: &Context<C>, commands: &mut Commands) -> FragmentId {
        commands.queue(|world: &mut World| {
            if !world.contains_resource::<Events<FragmentEvent<Data>>>() {
                EventRegistry::register_event::<FragmentEvent<Data>>(world);
            }
        });

        let data: Data = self.0.into();
        let id = commands
            .spawn(Leaf)
            .insert_begin_down(move |event, world| {
                world.send_event(FragmentEvent {
                    id: event.id,
                    data: data.clone(),
                });
            })
            .id();

        FragmentId::new(id)
    }
}
