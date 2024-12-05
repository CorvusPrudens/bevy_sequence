use crate::app::SequenceSets;
use crate::fragment::{BeginEvent, BeginKind, EndEvent, EndKind, SelectedFragments};
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

impl<T, Context, Data: Threaded> IntoFragment<Context, Data> for DataLeaf<T>
where
    Data: From<T> + Clone,
{
    fn into_fragment(self, _: &Context, commands: &mut Commands) -> FragmentId {
        commands.queue(|world: &mut World| {
            crate::app::add_systems_checked(
                world,
                bevy_app::prelude::PreUpdate,
                emit_leaves::<Data>.in_set(SequenceSets::Emit),
            );

            if !world.contains_resource::<Events<FragmentEvent<Data>>>() {
                EventRegistry::register_event::<FragmentEvent<Data>>(world);
            }
        });

        let entity = commands.spawn(DataLeaf::<Data>(self.0.into()));

        FragmentId::new(entity.id())
    }
}

fn emit_leaves<Data>(
    mut leaves: Query<(&DataLeaf<Data>, &mut FragmentState)>,
    mut writer: EventWriter<FragmentEvent<Data>>,
    selected_fragments: Res<SelectedFragments>,
    mut commands: Commands,
) where
    Data: Threaded + Clone,
{
    for fragment in selected_fragments.0.iter().copied() {
        if let Ok((leaf, mut state)) = leaves.get_mut(fragment) {
            let event = EventId::new();

            state.active_events.insert(event);
            state.triggered += 1;

            let id = IdPair {
                fragment: FragmentId(fragment),
                event,
            };

            commands.trigger_targets(
                BeginEvent {
                    id,
                    kind: BeginKind::Start,
                },
                fragment,
            );

            writer.send(FragmentEvent {
                id,
                data: leaf.0.clone(),
            });
        }
    }
}

fn respond_to_leaf(
    mut leaves: Query<&mut FragmentState, With<Leaf>>,
    mut reader: EventReader<FragmentEndEvent>,
    mut commands: Commands,
) {
    for event in reader.read() {
        if let Ok(mut state) = leaves.get_mut(event.0.fragment.0) {
            if state.active_events.remove(event.0.event) {
                state.completed += 1;

                commands.trigger_targets(
                    EndEvent {
                        id: event.0,
                        kind: EndKind::End,
                    },
                    event.0.fragment.0,
                );
            }
        }
    }
}
