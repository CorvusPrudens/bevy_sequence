use bevy::{log::LogPlugin, prelude::*};
use bevy_sequence::prelude::*;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), SequencePlugin))
        .add_systems(Startup, |mut commands: Commands| {
            info!("Starting up");
            spawn_root(count(), &mut commands);
        })
        .add_systems(Update, ping_pong)
        .run();
}

#[derive(Debug, Clone)]
struct Dialogue(String);

fn count() -> impl IntoFragment<Dialogue> {
    select(("one", "two", "three"), |mut tick: Local<usize>| {
        let value = *tick;
        *tick += 1;
        value
    })
    .limit(3)
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
        println!("{}", &event.data.0);
        writer.send(event.end());
    }
}
