use bevy::{log::LogPlugin, prelude::*};
use bevy_sequence::{combinators::save::SavedSequences, prelude::*};
use std::time::Duration;

const SAVE_FILE: &str = "target/sequence_data.json";

fn main() {
    App::new()
        .insert_resource(Persistent(Timer::new(
            Duration::from_secs(1),
            TimerMode::Repeating,
        )))
        .add_plugins((MinimalPlugins, LogPlugin::default(), SequencePlugin))
        .add_systems(
            Startup,
            |mut commands: Commands, mut saved: ResMut<SavedSequences>| {
                info!("Starting up");
                spawn_root(shopkeep().limit(2), &mut commands);

                if let Ok(data) = std::fs::read(SAVE_FILE) {
                    let data = serde_json::from_slice(&data).unwrap();
                    *saved = data;
                }
            },
        )
        .add_systems(Update, ping_pong)
        .add_systems(Last, save_sequences)
        .run();
}

#[derive(Debug, Clone)]
struct Dialogue(String);

fn shopkeep() -> impl IntoFragment<Dialogue> {
    (
        (
            "First time, eh?",
            "Let's give you the rundown:",
            "you pay me, we got a deal.",
        )
            .once()
            .or("Well then..."),
        "What are you buying?",
    )
        .always()
        .save_as("shopkeep_greet")
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
        println!("{}", &event.data.0);
        writer.send(event.end());
    }
}

#[derive(Resource)]
struct Persistent(Timer);

fn save_sequences(mut pers: ResMut<Persistent>, time: Res<Time>, saved: Res<SavedSequences>) {
    pers.0.tick(time.delta());

    if pers.0.just_finished() {
        let data = serde_json::to_vec_pretty(saved.as_ref()).unwrap();
        std::fs::write(SAVE_FILE, &data).unwrap();
    }
}
