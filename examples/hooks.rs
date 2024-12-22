use bevy::{log::LogPlugin, prelude::*};
use bevy_sequence::prelude::*;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default(), SequencePlugin))
        .add_systems(Startup, |mut commands: Commands| {
            info!("Starting up");

            spawn_root_with(scene(), &mut commands, None);
        })
        .add_systems(Update, ping_pong)
        .run();
}

#[derive(Debug, Clone)]
struct Dialogue(&'static str);

fn scene() -> impl IntoFragment<Dialogue, Option<Entity>> {
    (
        "Hello, Alice!".on_start2(|| println!("Hello, world!")),
        "Hey Bob...".on_start2(|ctx: InRef<Option<Entity>>| println!("ctx: {ctx:?}")),
        "Crazy weather we're having, huh?".on_start2(
            |mut ctx: InMut<Option<Entity>>, mut commands: Commands| {
                *ctx = Some(commands.spawn_empty().id())
            },
        ),
    )
        .always()
        .once()
}

impl<C> IntoFragment<Dialogue, C> for &'static str {
    fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
        <_ as IntoFragment<Dialogue, C>>::into_fragment(
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
        println!("{}", &event.data.0);
        writer.send(event.end());
    }
}
