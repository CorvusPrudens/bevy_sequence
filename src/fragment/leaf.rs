use crate::fragment::event::{BeginStage, StageEvent};
use crate::prelude::*;
use bevy_ecs::event::EventRegistry;
use bevy_ecs::prelude::*;

use super::event::OnBeginUp;

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

impl<T, Data: Threaded, Context> IntoFragment<Data, Context> for DataLeaf<T>
where
    Data: From<T> + Clone,
{
    fn into_fragment(self, _: &Context, commands: &mut Commands) -> FragmentId {
        commands.queue(|world: &mut World| {
            // crate::app::add_systems_checked(
            //     world,
            //     bevy_app::prelude::PreUpdate,
            //     emit_leaves::<Data>.in_set(SequenceSets::Emit),
            // );

            if !world.contains_resource::<Events<FragmentEvent<Data>>>() {
                EventRegistry::register_event::<FragmentEvent<Data>>(world);
            }
        });

        let data: Data = self.0.into();
        let emitter = commands.register_system(
            move |input: In<StageEvent<BeginStage>>,
                  mut writer: EventWriter<FragmentEvent<Data>>| {
                writer.send(FragmentEvent {
                    id: input.0.id,
                    data: data.clone(),
                });
            },
        );
        let mut entity = commands.spawn(Leaf);
        let id = entity.id();
        entity
            .entry::<OnBeginUp>()
            .or_default()
            .and_modify(move |mut ob| ob.0.push(emitter));

        FragmentId::new(id)
    }
}
