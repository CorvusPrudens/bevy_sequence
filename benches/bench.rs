use bevy::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use bevy_sequence::{fragment2::*, FragmentId};
use criterion::{criterion_group, criterion_main, Criterion};
use std::{hint::black_box, sync::atomic::AtomicBool};

#[derive(Debug, Clone)]
struct Dialogue(&'static str);

#[derive(Component)]
struct Context;

fn scene() -> impl IntoFragment<Context, Dialogue> {
    (
        ("Hello, Alice!", "hey"),
        "Hey Bob...",
        ("Hello, Alice!", "hey"),
        "Hey Bob...",
        "Crazy weather we're having, huh?",
    )
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
        .eval(|| true)
}

impl IntoFragment<Context, Dialogue> for &'static str {
    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> bevy_sequence::FragmentId {
        <_ as IntoFragment<_, Dialogue>>::into_fragment(
            DataLeaf::new(Dialogue(self)),
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
        writer.send(event.end());
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("simple spawn", |b| {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SequencePlugin));

        let world = app.world_mut();

        b.iter(|| {
            let mut commands = world.commands();

            for i in 0..100 {
                spawn_root(black_box(scene()), Context, &mut commands);
            }

            drop(commands);
            world.flush();
        })
    });

    // Constantly evaluate sequences.
    c.bench_function("selection control", |b| {
        let mut world = World::new();
        let mut commands = world.commands();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, ping_pong)
            .add_systems(Startup, |mut commands: Commands| {
                spawn_root(scene(), Context, &mut commands);
            });

        b.iter(|| {
            app.update();
        })
    });

    c.bench_function("selection one", |b| {
        let mut world = World::new();
        let mut commands = world.commands();

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SequencePlugin))
            .add_systems(Update, ping_pong)
            .add_systems(Startup, |mut commands: Commands| {
                spawn_root(scene(), Context, &mut commands);
            });

        b.iter(|| {
            app.update();
        })
    });

    c.bench_function("selection thousand", |b| {
        let mut world = World::new();
        let mut commands = world.commands();

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SequencePlugin))
            .add_systems(Update, ping_pong)
            .add_systems(Startup, |mut commands: Commands| {
                for _ in 0..1000 {
                    spawn_root(scene(), Context, &mut commands);
                }
            });

        b.iter(|| {
            app.update();
        })
    });

    c.bench_function("selection thousand nested", |b| {
        let mut world = World::new();
        let mut commands = world.commands();

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SequencePlugin))
            .add_systems(Update, ping_pong)
            .add_systems(Startup, |mut commands: Commands| {
                for _ in 0..1000 {
                    spawn_root(nested(), Context, &mut commands);
                }
            });

        b.iter(|| {
            app.update();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
