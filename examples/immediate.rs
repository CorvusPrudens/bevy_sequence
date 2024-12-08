use bevy::{log::LogPlugin, prelude::*};
use bevy_sequence::prelude::*;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), SequencePlugin))
        .add_systems(Startup, |mut commands: Commands| {
            info!("Starting up");
            spawn_root(shopkeep().limit(2), &mut commands);
        })
        .add_systems(Update, ping_pong)
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
        .eval(|| true)
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
