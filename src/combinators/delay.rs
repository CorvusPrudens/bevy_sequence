use crate::{
    fragment::event::{EndStage, InsertEndDown},
    prelude::*,
};
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_time::{Time, Timer, TimerMode};
use std::{marker::PhantomData, time::Duration};

#[derive(Component, Clone)]
pub struct AfterSystem {
    id: SystemId,
    timer: Timer,
    paused: bool,
}

impl AfterSystem {
    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn unpause(&mut self) {
        self.paused = false;
    }
}

/// Run a one-shot system after the specified delay.
pub fn run_after<M>(
    delay: Duration,
    system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    commands: &mut Commands,
) {
    let system = commands.register_system(system);
    commands.spawn(AfterSystem {
        id: system,
        timer: Timer::new(delay, TimerMode::Once),
        paused: false,
    });
}

pub(super) fn manage_delay(
    mut q: Query<(Entity, &mut AfterSystem)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut sys) in q.iter_mut() {
        if !sys.paused {
            sys.timer.tick(time.delta());
            if sys.timer.finished() {
                commands.run_system(sys.id);
                commands.entity(entity).despawn();
            }
        }
    }
}

pub struct Delay<F, S, M> {
    fragment: F,
    system: S,
    duration: Duration,
    _marker: PhantomData<fn() -> M>,
}

impl<F, S, M> Delay<F, S, M> {
    pub fn new(fragment: F, duration: Duration, system: S) -> Self {
        Self {
            fragment,
            duration,
            system,
            _marker: PhantomData,
        }
    }
}

impl<F, S, D, C, M> IntoFragment<D, C> for Delay<F, S, M>
where
    F: IntoFragment<D, C>,
    D: Threaded,
    S: IntoSystem<(), (), M> + 'static,
{
    fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);

        let system = commands.register_system(self.system);
        commands
            .entity(id.entity())
            .insert_end_down(move |stage, world| {
                if matches!(stage.stage, EndStage::End) {
                    world.commands().spawn(AfterSystem {
                        id: system,
                        timer: Timer::new(self.duration, TimerMode::Once),
                        paused: false,
                    });
                }
            });

        id
    }
}
