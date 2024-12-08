use bevy::{ecs::schedule::Stepping, log::LogPlugin, prelude::*};
use bevy_sequence::prelude::*;
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

                for _ in 0..1000 {
                    spawn_root(scene(), &mut commands);
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

fn scene() -> impl IntoFragment<Dialogue> {
    (
        ("Hello, Alice!", "hey"),
        "Hey Bob...",
        "Crazy weather we're having, huh?",
    )
        .eval(|| true)
}

fn nested() -> impl IntoFragment<Dialogue> {
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
        .eval(|| true)
}

impl IntoFragment<Dialogue> for &'static str {
    fn into_fragment(self, context: &(), commands: &mut Commands) -> FragmentId {
        <_ as IntoFragment<Dialogue>>::into_fragment(
            bevy_sequence::fragment::DataLeaf::new(Dialogue(self.into())),
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
