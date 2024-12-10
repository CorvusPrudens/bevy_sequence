use super::{FragmentState, Root, SelectedFragments};
use crate::prelude::FragmentId;
use bevy_ecs::{prelude::*, system::SystemId, traversal::Traversal};
use bevy_hierarchy::Parent;
use std::{marker::PhantomData, sync::Arc};

/// A unique ID generated for every emitted event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EventId(u64);

impl EventId {
    pub fn new() -> Self {
        use rand::prelude::*;

        Self(rand::thread_rng().gen())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActiveEvents(Vec<EventId>);

impl core::ops::Deref for ActiveEvents {
    type Target = [EventId];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ActiveEvents {
    pub fn new(events: Vec<EventId>) -> Self {
        Self(events)
    }

    /// Remove an ID by value.
    ///
    /// If the ID existed in the set and was removed,
    /// this returns true.
    pub fn remove(&mut self, id: EventId) -> bool {
        if let Some(index) = self.iter().position(|e| *e == id) {
            self.0.swap_remove(index);
            true
        } else {
            false
        }
    }

    pub fn insert(&mut self, id: EventId) {
        self.0.push(id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdPair {
    pub fragment: FragmentId,
    pub event: EventId,
}

#[derive(Debug, Event, Clone)]
pub struct FragmentEvent<Data> {
    pub id: IdPair,
    pub data: Data,
}

impl<Data> FragmentEvent<Data> {
    pub fn end(&self) -> FragmentEndEvent {
        FragmentEndEvent(self.id)
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct FragmentEndEvent(pub IdPair);

#[derive(Debug, Clone, Copy, Component)]
#[component(storage = "SparseSet")]
pub struct StageEvent<Stage> {
    pub id: IdPair,
    pub stage: Stage,
}

impl<Stage> Event for StageEvent<Stage>
where
    Stage: Send + Sync + 'static,
{
    const AUTO_PROPAGATE: bool = true;
    type Traversal = &'static Parent;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeginStage {
    Start,
    Visit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndStage {
    End,
    Visit,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct EventPath {
    pub child: Option<Entity>,
    pub end: EndStage,
    pub begin: BeginStage,
}

impl Default for EventPath {
    fn default() -> Self {
        EventPath {
            child: None,
            end: EndStage::Visit,
            begin: BeginStage::Visit,
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
#[component(storage = "SparseSet")]
pub struct StageEventDown<Stage> {
    pub id: IdPair,
    _marker: PhantomData<Stage>,
}

impl<Stage> Event for StageEventDown<Stage>
where
    Stage: Send + Sync + 'static,
{
    const AUTO_PROPAGATE: bool = true;
    type Traversal = &'static EventPath;
}

impl Traversal for &'static EventPath {
    fn traverse(item: Self::Item<'_>) -> Option<Entity> {
        item.child
    }
}

macro_rules! callback {
    ($name:ident, $stage:ty, $tr:ident, $method:ident) => {
        #[derive(Clone, Component)]
        pub struct $name(
            pub Vec<Arc<dyn Fn(StageEvent<$stage>, &mut World) + Send + Sync + 'static>>,
        );

        impl Default for $name {
            fn default() -> Self {
                $name(vec![])
            }
        }

        // impl Component for $name {
        //     const STORAGE_TYPE: StorageType = StorageType::Table;

        //     fn register_component_hooks(hooks: &mut bevy_ecs::component::ComponentHooks) {
        //         hooks.on_remove(|mut world, entity, _| {
        //             let entity = world.entity(entity);
        //             let items = entity.components::<&$name>().0.clone();
        //             let mut commands = world.commands();

        //             for item in items {
        //                 commands.unregister_system(item);
        //             }
        //         });
        //     }
        // }

        pub trait $tr {
            fn $method<F>(&mut self, hook: F) -> &mut Self
            where
                F: Fn(StageEvent<$stage>, &mut World) + Send + Sync + 'static;
        }

        impl $tr for EntityCommands<'_> {
            fn $method<F>(&mut self, hook: F) -> &mut Self
            where
                F: Fn(StageEvent<$stage>, &mut World) + Send + Sync + 'static,
            {
                self.entry::<$name>()
                    .or_default()
                    .and_modify(move |mut ob| ob.0.push(Arc::new(hook)));
                self
            }
        }
    };
}

callback!(OnBeginUp, BeginStage, InsertBeginUp, insert_begin_up);
callback!(OnBeginDown, BeginStage, InsertBeginDown, insert_begin_down);
callback!(OnEndUp, EndStage, InsertEndUp, insert_end_up);
callback!(OnEndDown, EndStage, InsertEndDown, insert_end_down);

pub struct MapContext<Stage> {
    pub target: Entity,
    pub child: Option<Entity>,
    pub event: StageEvent<Stage>,
}

#[derive(Clone)]
pub enum MapFn<Stage: 'static> {
    Function(Arc<dyn Fn(MapContext<Stage>) -> StageEvent<Stage> + Send + Sync + 'static>),
    System(SystemId<In<MapContext<Stage>>, StageEvent<Stage>>),
}

impl<Stage> MapFn<Stage> {
    fn call(&self, world: &mut World, context: MapContext<Stage>) -> StageEvent<Stage> {
        match self {
            MapFn::Function(function) => function(context),
            MapFn::System(sys) => world.run_system_with_input(*sys, context).unwrap(),
        }
    }
}

impl<Stage: 'static> MapFn<Stage> {
    pub fn function<F>(function: F) -> Self
    where
        F: Fn(MapContext<Stage>) -> StageEvent<Stage> + Send + Sync + 'static,
    {
        Self::Function(Arc::new(function))
    }
}

impl<Stage: Send + 'static> Component for MapFn<Stage> {
    const STORAGE_TYPE: bevy_ecs::component::StorageType = bevy_ecs::component::StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy_ecs::component::ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let map = world.get::<MapFn<Stage>>(entity).unwrap();
            if let MapFn::System(system) = map {
                let system = *system;
                world.commands().unregister_system(system);
            }
        });
    }
}

fn begin_recursive(
    node: Entity,
    child_node: Option<Entity>,
    mut event: StageEvent<BeginStage>,
    world: &mut World,
) -> Option<()> {
    let child = world.get_entity(node).ok()?;
    let (parent_id, on_begin, on_begin_down, root, map) = child.get_components::<AnyOf<(
        &Parent,
        &OnBeginUp,
        &OnBeginDown,
        &Root,
        &MapFn<BeginStage>,
    )>>()?;

    let (parent_id, on_begin, on_begin_down, root, map) = (
        parent_id.map(|p| p.get()),
        on_begin.cloned(),
        on_begin_down.cloned(),
        root.cloned(),
        map.cloned(),
    );

    if let Some(map) = map {
        event = map.call(
            world,
            MapContext {
                target: node,
                child: child_node,
                event,
            },
        );
    }

    for system in on_begin.iter().flat_map(|o| o.0.iter()) {
        // world.run_system_with_input(*system, event).unwrap();

        (system)(event, world);
    }

    let mut child = world.get_entity_mut(node).ok()?;
    let mut state = child.get_mut::<FragmentState>()?;

    if matches!(event.stage, BeginStage::Start) {
        state.triggered += 1;
        state.active = true;
    }
    state.active_events.insert(event.id.event);

    if root.is_none() {
        if let Some(parent) = parent_id {
            begin_recursive(parent, Some(node), event, world);
        }
    }

    for system in on_begin_down.iter().flat_map(|o| o.0.iter()) {
        // world.run_system_with_input(*system, event).unwrap();
        (system)(event, world);
    }

    Some(())
}

pub(crate) fn begin_world(world: &mut World) {
    let targets = world
        .get_resource::<SelectedFragments>()
        .map(|sf| sf.0.clone())
        .unwrap_or_default();

    for target in targets {
        // traverse up and down the tree
        begin_recursive(
            target,
            None,
            StageEvent {
                stage: BeginStage::Start,
                id: IdPair {
                    fragment: FragmentId::new(target),
                    event: EventId::new(),
                },
            },
            world,
        );
    }
}

fn end_recursive(
    node: Entity,
    child_node: Option<Entity>,
    mut event: StageEvent<EndStage>,
    world: &mut World,
) -> Option<()> {
    let child = world.get_entity(node).ok()?;
    let (parent_id, on_end, on_end_down, root, map) =
        child
            .get_components::<AnyOf<(&Parent, &OnEndUp, &OnEndDown, &Root, &MapFn<EndStage>)>>()?;

    let (parent_id, on_end, on_end_down, root, map) = (
        parent_id.map(|p| p.get()),
        on_end.cloned(),
        on_end_down.cloned(),
        root.cloned(),
        map.cloned(),
    );

    let mut child = world.get_entity_mut(node).ok()?;
    let mut state = child.get_mut::<FragmentState>()?;

    if state.active_events.remove(event.id.event) {
        if let Some(map) = map {
            event = map.call(
                world,
                MapContext {
                    target: node,
                    child: child_node,
                    event,
                },
            );
        }

        for system in on_end.iter().flat_map(|o| o.0.iter()) {
            // world.run_system_with_input(*system, event).unwrap();
            (system)(event, world);
        }

        let mut child = world.get_entity_mut(node).ok()?;
        let mut state = child.get_mut::<FragmentState>()?;

        if matches!(event.stage, EndStage::End) {
            state.completed += 1;
            state.active = false;
        }

        if root.is_none() {
            if let Some(parent) = parent_id {
                end_recursive(parent, Some(node), event, world);
            }
        }

        for system in on_end_down.iter().flat_map(|o| o.0.iter()) {
            // world.run_system_with_input(*system, event).unwrap();
            (system)(event, world);
        }
    }

    Some(())
}

pub(crate) fn end_world(mut reader: EventReader<FragmentEndEvent>, mut commands: Commands) {
    let end_events: Vec<_> = reader.read().copied().collect();

    commands.queue(move |world: &mut World| {
        for target in end_events {
            end_recursive(
                target.0.fragment.0,
                None,
                StageEvent {
                    stage: EndStage::End,
                    id: target.0,
                },
                world,
            );
        }
    });
}
