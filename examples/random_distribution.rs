use bevy::{log::LogPlugin, prelude::*};
use bevy_sequence::prelude::*;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), SequencePlugin))
        .add_systems(Startup, |mut commands: Commands| {
            info!("Starting up");
            spawn_root(
                weighted().limit(15).or(random().limit(15)).always(),
                &mut commands,
            );
        })
        .add_systems(Update, ping_pong)
        .run();
}

#[derive(Debug, Clone)]
struct Dialogue(String);

fn random() -> impl IntoFragment<Dialogue> {
    choice((("a0", "a1"), "b", "c"))
}

fn weighted() -> impl IntoFragment<Dialogue> {
    distribution(
        (
            "most probable",
            "not very probable",
            "also not very probable",
        ),
        [5, 2, 2],
    )
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
