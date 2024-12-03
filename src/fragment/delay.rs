use super::{Fragment, IntoFragment, Threaded};
use bevy::{ecs::system::SystemId, prelude::*};
use std::time::Duration;

#[derive(Component, Clone)]
pub(crate) struct AfterSystem(SystemId, Timer);

/// Run a one-shot system after the specified delay.
pub fn run_after<M>(
    delay: Duration,
    system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    commands: &mut Commands,
) {
    let system = commands.register_system(system);
    commands.spawn(AfterSystem(system, Timer::new(delay, TimerMode::Once)));
}

pub(crate) fn manage_delay(
    mut q: Query<(Entity, &mut AfterSystem)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut sys) in q.iter_mut() {
        sys.1.tick(time.delta());
        if sys.1.finished() {
            commands.run_system(sys.0);
            commands.entity(entity).despawn();
        }
    }
}

pub struct Delay<F, S> {
    fragment: F,
    system: S,
    duration: Duration,
}

impl<F, S> Delay<F, S> {
    pub fn new(fragment: F, duration: Duration, system: S) -> Self {
        Self {
            fragment,
            duration,
            system,
        }
    }
}

impl<F, S, D, C> IntoFragment<C, D> for Delay<F, S>
where
    F: IntoFragment<C, D>,
    D: Threaded,
    C: Threaded,
    S: System<In = (), Out = ()> + 'static,
{
    type Fragment = Delay<F::Fragment, SystemId>;

    fn into_fragment(
        self,
        context: &C,
        commands: &mut Commands,
    ) -> (Self::Fragment, super::FragmentNode) {
        let (fragment, n) = self.fragment.into_fragment(context, commands);

        (
            Delay {
                fragment,
                duration: self.duration,
                system: commands.register_system(self.system),
            },
            n,
        )
    }
}

impl<F, D, C> Fragment<C, D> for Delay<F, SystemId>
where
    F: Fragment<C, D>,
    D: Threaded,
    C: Threaded,
{
    fn start(
        &mut self,
        context: &C,
        id: crate::FragmentId,
        state: &mut crate::evaluate::FragmentStates,
        writer: &mut EventWriter<crate::FragmentEvent<D>>,
        commands: &mut Commands,
    ) -> super::Start {
        self.fragment.start(context, id, state, writer, commands)
    }

    fn end(
        &mut self,
        context: &C,
        id: crate::FragmentId,
        state: &mut crate::evaluate::FragmentStates,
        commands: &mut Commands,
    ) -> super::End {
        let end = self.fragment.end(context, id, state, commands);

        if end.exited() {
            commands.spawn(AfterSystem(
                self.system,
                Timer::new(self.duration, TimerMode::Once),
            ));
        }

        end
    }

    fn id(&self) -> &crate::FragmentId {
        self.fragment.id()
    }
}
