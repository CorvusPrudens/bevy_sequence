use bevy::{ecs::schedule::Stepping, log::LogPlugin, prelude::*};
use bevy_sequence::{fragment2::*, FragmentId};
use std::time::Duration;

fn main() {
    App::new()
        .insert_resource(Stepping::new())
        .insert_resource(Stepping::new())
        .add_plugins((MinimalPlugins, LogPlugin::default(), SequencePlugin))
        .add_systems(
            Startup,
            |mut commands: Commands, mut stepping: ResMut<Stepping>| {
                // stepping
                //     .add_schedule(FragmentUpdate)
                //     .add_schedule(PostUpdate)
                //     .enable();

                info!("Starting up");

                for _ in 0..100 {
                    spawn_root(nested(), Context, &mut commands);
                }
            },
        )
        .insert_resource(StepTime(Timer::new(
            Duration::from_secs(1),
            TimerMode::Repeating,
        )))
        .add_systems(Update, (stepper, ping_pong))
        .run();
}

#[derive(Debug, Clone)]
struct Dialogue(String);

#[derive(Component)]
struct Context;

fn scene() -> impl IntoFragment<Context, Dialogue> {
    (
        ("Hello, Alice!", "hey"),
        "Hey Bob...",
        "Crazy weather we're having, huh?",
    )
        .once()
        .eval(|| true)
}

fn nested() -> impl IntoFragment<Context, Dialogue> {
    (
        (
            (("Hey Bob!", "Hey, Alice 1!"), ("Hey Bob!", "Hey, Alice 2!")),
            (("Hey Bob!", "Hey, Alice 3!"), ("Hey Bob!", "Hey, Alice 4!")),
        ),
        (
            (("Hey Bob!", "Hey, Alice 5!"), ("Hey Bob!", "Hey, Alice 6!")),
            (("Hey Bob!", "Hey, Alice 7!"), ("Hey Bob!", "Hey, Alice 8!")),
        ),
    )
        .once()
        .eval(|| true)
}

impl IntoFragment<Context, Dialogue> for &'static str {
    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> bevy_sequence::FragmentId {
        <_ as IntoFragment<_, Dialogue>>::into_fragment(
            DataLeaf::new(Dialogue(self.into())),
            context,
            commands,
        )
    }
}

fn ping_pong(
    mut reader: EventReader<FragmentEvent<Dialogue>>,
    mut writer: EventWriter<FragmentEndEvent>,
) {
    for event in reader.read() {
        // println!("{}", &event.data.0);
        writer.send(event.end());
    }
}

#[derive(Resource)]
struct StepTime(Timer);

fn stepper(mut step: ResMut<StepTime>, mut stepping: ResMut<Stepping>, time: Res<Time>) {
    step.0.tick(time.delta());

    if step.0.just_finished() {
        // stepping.step_frame();
    }
}
