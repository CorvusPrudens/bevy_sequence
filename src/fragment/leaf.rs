use super::Context;
use super::event::InsertBeginDown;
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

#[macro_export]
macro_rules! impl_leaf {
    ($self:ident) => {
        impl bevy_sequence::prelude::IntoFragment<$self, ()> for $self {
            fn into_fragment(
                self,
                context: &bevy_sequence::prelude::Context<()>,
                commands: &mut bevy::prelude::Commands,
            ) -> bevy_sequence::prelude::FragmentId {
                <_ as bevy_sequence::prelude::IntoFragment<$self, ()>>::into_fragment(
                    bevy_sequence::fragment::DataLeaf::new(self),
                    context,
                    commands,
                )
            }
        }
    };

    ($self:ident, $ctx:ty) => {
        impl bevy_sequence::prelude::IntoFragment<$self, $ctx> for $self {
            fn into_fragment(
                self,
                context: &bevy_sequence::prelude::Context<$ctx>,
                commands: &mut bevy::prelude::Commands,
            ) -> bevy_sequence::prelude::FragmentId {
                <_ as bevy_sequence::prelude::IntoFragment<$self, $ctx>>::into_fragment(
                    bevy_sequence::fragment::DataLeaf::new(self),
                    context,
                    commands,
                )
            }
        }
    };
}
