use super::{End, Fragment, FragmentNode, IntoFragment, Start, Threaded};
use crate::evaluate::FragmentStates;
use crate::{FragmentEvent, FragmentId};
use bevy::ecs::system::SystemId;
use bevy::prelude::*;

pub struct Dynamic<S> {
    id: FragmentId,
    system: S,
}

/// A dynamic text fragment.
///
/// This takes any system that outputs a string.
pub fn dynamic<S, M, O>(system: S) -> Dynamic<S::System>
where
    S: IntoSystem<(), O, M>,
{
    let id = todo!();
    Dynamic {
        id,
        system: IntoSystem::into_system(system),
    }
}

impl<S, Context, Data> IntoFragment<Context, Data> for Dynamic<S>
where
    Data: Threaded,
    S: System<In = ()>,
    S::Out: Send + Sync + 'static + Into<Data>,
{
    type Fragment = Dynamic<SystemId>;

    fn into_fragment(self, _: &Context, commands: &mut Commands) -> (Self::Fragment, FragmentNode) {
        let id = self.id;
        (
            Dynamic {
                id,
                system: commands.register_system(self.system.pipe(
                    move |data: In<S::Out>, mut writer: EventWriter<FragmentEvent<Data>>| {
                        writer.send(FragmentEvent {
                            id,
                            data: data.0.into(),
                        });
                    },
                )),
            },
            FragmentNode::leaf(id),
        )
    }
}

impl<Context, Data> Fragment<Context, Data> for Dynamic<SystemId>
where
    Data: Threaded,
{
    fn start(
        &mut self,
        _: &Context,
        id: FragmentId,
        state: &mut FragmentStates,
        _writer: &mut EventWriter<FragmentEvent<Data>>,
        commands: &mut Commands,
    ) -> Start {
        if id == self.id {
            commands.run_system(self.system);

            let state = state.update(id);
            state.triggered += 1;
            state.active = true;

            Start::Entered
        } else {
            Start::Unvisited
        }
    }

    fn end(
        &mut self,
        _: &Context,
        id: FragmentId,
        state: &mut FragmentStates,
        _commands: &mut Commands,
    ) -> End {
        if id == self.id {
            let state = state.update(id);
            state.completed += 1;
            state.active = false;

            End::Exited
        } else {
            End::Unvisited
        }
    }

    fn id(&self) -> &FragmentId {
        &self.id
    }
}
