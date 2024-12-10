use bevy::{log::LogPlugin, prelude::*};
use bevy_sequence::prelude::*;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), SequencePlugin))
        .add_systems(Startup, |mut commands: Commands| {
            info!("Starting up");

            spawn_root(scene(), &mut commands);
        })
        .add_systems(Update, ping_pong)
        .run();
}

#[derive(Debug, Clone)]
struct Dialogue(&'static str);

fn scene() -> impl IntoFragment<Dialogue> {
    (
        ("Hello, Alice!", "hey"),
        "Hey Bob...",
        ("Hello, Alice!", "hey"),
        "Hey Bob...",
        "Crazy weather we're having, huh?",
    )
        .always()
}

impl IntoFragment<Dialogue> for &'static str {
    fn into_fragment(self, context: &(), commands: &mut Commands) -> FragmentId {
        <_ as IntoFragment<Dialogue>>::into_fragment(
            bevy_sequence::fragment::DataLeaf::new(Dialogue(self)),
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
