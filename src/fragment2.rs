#![allow(dead_code)]

use crate::evaluate::{Evaluate, Evaluation};
use crate::FragmentUpdate;
use crate::{fragment::Threaded, FragmentId};
use bevy::app::MainScheduleOrder;
use bevy::ecs::component::StorageType;
use bevy::ecs::event::EventRegistry;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use bevy::utils::{all_tuples_with_size, HashSet};
use std::any::TypeId;
use std::marker::PhantomData;

pub struct SequencePlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SequenceSets {
    /// Evaluate node scores.
    Evaluate,

    /// Iterate over all nodes and determine which, if any, should be selected.
    Select,

    /// Emit events from the selected nodes.
    Emit,

    /// Respond to end events.
    Respond,
}

impl Plugin for SequencePlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(FragmentUpdate);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_before(PreUpdate, FragmentUpdate);

        app.insert_resource(AddedSystems(Default::default()))
            .insert_resource(SelectedFragments::default())
            .add_event::<FragmentEndEvent>()
            .add_systems(
                FragmentUpdate,
                (
                    (update_sequence_items, custom_evals, evaluate_limits)
                        .in_set(SequenceSets::Evaluate),
                    select_fragments.in_set(SequenceSets::Select),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    respond_to_leaf.in_set(SequenceSets::Respond),
                    clear_evals.after(SequenceSets::Respond),
                ),
            )
            .configure_sets(
                FragmentUpdate,
                (
                    SequenceSets::Select.after(SequenceSets::Evaluate),
                    SequenceSets::Emit.after(SequenceSets::Select),
                ),
            );
    }
}

pub trait IntoFragment<Context, Data: Threaded> {
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId;
}

pub fn spawn_root<Context: Component, Data: Threaded>(
    fragment: impl IntoFragment<Context, Data>,
    context: Context,
    commands: &mut Commands,
) {
    let root = fragment.into_fragment(&context, commands);
    commands.entity(root.0).insert((context, Root));
}

impl<T> FragmentExt for T {}

pub trait FragmentExt: Sized {
    /// Limit this fragment to `n` triggers.
    fn limit(self, n: usize) -> Limit<Self> {
        Limit::new(self, n)
    }

    /// Set this fragment's limit to 1.
    fn once(self) -> Limit<Self> {
        self.limit(1)
    }

    /// Wrap this fragment in an evaluation.
    fn eval<S, O, M>(self, system: S) -> Evaluated<Self, S, O, M>
    where
        S: IntoSystem<In<FragmentId>, O, M> + 'static,
        O: Evaluate + 'static,
    {
        Evaluated {
            fragment: self,
            evaluation: system,
            _marker: PhantomData,
        }
    }
}

/// A unique ID generated for every emitted event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(u64);

impl EventId {
    pub fn new() -> Self {
        use rand::prelude::*;

        Self(rand::thread_rng().gen())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
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

#[derive(Debug, Component, Default, Clone, PartialEq, Eq)]
pub struct FragmentState {
    pub triggered: usize,
    pub completed: usize,
    pub active_events: ActiveEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdPair {
    fragment: FragmentId,
    event: EventId,
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

#[derive(Debug, Event, Clone, Copy)]
pub struct FragmentEndEvent(IdPair);

/// An entity representing a sequence fragment.
#[derive(Debug, Default, Component)]
#[require(Evaluation, FragmentState)]
pub struct Fragment;

/// A root fragment.
#[derive(Debug, Default, Component)]
#[require(Fragment)]
pub struct Root;

/// A leaf fragment.
///
/// Leaf fragments are nodes that emit [FragmentEvent]s.
#[derive(Debug, Default, Component)]
#[require(Fragment)]
pub struct Leaf;

/// A leaf node that simply emits its contained value.
#[derive(Debug, Component)]
#[require(Leaf)]
pub struct DataLeaf<T>(T);

impl<T> DataLeaf<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

#[derive(Resource)]
struct AddedSystems(HashSet<TypeId>);

trait AddSystemsChecked: Sized {
    /// Queues inserting systems into a schedule.
    ///
    /// This will only insert a set of systems into a given schedule once.
    fn add_systems_checked<M, S, C>(&mut self, schedule: S, systems: C)
    where
        S: ScheduleLabel,
        C: IntoSystemConfigs<M> + Send + 'static;
}

impl<'w, 's> AddSystemsChecked for Commands<'w, 's> {
    fn add_systems_checked<M, S, C>(&mut self, schedule: S, systems: C)
    where
        S: ScheduleLabel,
        C: IntoSystemConfigs<M> + Send + 'static,
    {
        self.queue(|world: &mut World| {
            let id = TypeId::of::<(S, C)>();
            let mut pairs = world.get_resource_or_insert_with(|| AddedSystems(Default::default()));

            if pairs.0.insert(id) {
                let mut schedules = world.resource_mut::<Schedules>();
                schedules.add_systems(schedule, systems);
            }
        });
    }
}

impl<T, Context, Data: Threaded> IntoFragment<Context, Data> for DataLeaf<T>
where
    Data: From<T> + Clone,
{
    fn into_fragment(self, _: &Context, commands: &mut Commands) -> FragmentId {
        commands.add_systems_checked(
            FragmentUpdate,
            emit_leaves::<Data>.in_set(SequenceSets::Emit),
        );
        commands.queue(|world: &mut World| {
            if !world.contains_resource::<Events<FragmentEvent<Data>>>() {
                EventRegistry::register_event::<FragmentEvent<Data>>(world);
            }
        });

        let entity = commands.spawn(DataLeaf::<Data>(self.0.into()));

        FragmentId::new(entity.id())
    }
}

fn emit_leaves<Data>(
    mut leaves: Query<(&DataLeaf<Data>, &mut FragmentState)>,
    mut writer: EventWriter<FragmentEvent<Data>>,
    selected_fragments: Res<SelectedFragments>,
    mut commands: Commands,
) where
    Data: Threaded + Clone,
{
    info!("selected: {selected_fragments:?}");

    for fragment in selected_fragments.0.iter().copied() {
        if let Ok((leaf, mut state)) = leaves.get_mut(fragment) {
            let event = EventId::new();

            state.active_events.insert(event);
            state.triggered += 1;

            let id = IdPair {
                fragment: FragmentId(fragment),
                event,
            };

            commands.trigger_targets(
                BeginEvent {
                    id,
                    kind: BeginKind::Start,
                },
                fragment,
            );

            writer.send(FragmentEvent {
                id,
                data: leaf.0.clone(),
            });
        }
    }
}

fn respond_to_leaf(
    mut leaves: Query<&mut FragmentState, With<Leaf>>,
    mut reader: EventReader<FragmentEndEvent>,
    mut commands: Commands,
) {
    for event in reader.read() {
        if let Ok(mut state) = leaves.get_mut(event.0.fragment.0) {
            if state.active_events.remove(event.0.event) {
                state.completed += 1;

                commands.trigger_targets(
                    EndEvent {
                        id: event.0,
                        kind: EndKind::End,
                    },
                    event.0.fragment.0,
                );
            }
        }
    }
}

fn clear_evals(mut evals: Query<&mut Evaluation>) {
    for mut eval in evals.iter_mut() {
        *eval = Default::default();
    }
}

#[derive(Component)]
#[require(Fragment)]
struct Sequence;

fn update_sequence_items(
    q: Query<(&Children, &FragmentState), With<Sequence>>,
    mut children: Query<(&mut Evaluation, &FragmentState)>,
) {
    for (seq, outer_state) in q.iter() {
        info!("outer: {outer_state:?}");

        let inactive = outer_state.active_events.is_empty();

        // look for the first item that has finished equal to the container
        let mut first_selected = false;
        for child in seq.iter() {
            let Ok((mut eval, state)) = children.get_mut(*child) else {
                continue;
            };

            if inactive
                && !first_selected
                && state.active_events.is_empty()
                && state.completed <= outer_state.completed
            {
                first_selected = true;
                eval.merge(true.evaluate());

                continue;
            }

            eval.merge(false.evaluate());
        }
    }
}

#[derive(Debug, Event)]
pub struct BeginEvent {
    pub id: IdPair,
    pub kind: BeginKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BeginKind {
    Start,
    Visit,
}

#[derive(Debug, Event)]
pub struct EndEvent {
    pub id: IdPair,
    pub kind: EndKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EndKind {
    End,
    Visit,
}

fn spawn_sequence(children: &[Entity], commands: &mut Commands) -> FragmentId {
    let id = commands.spawn(Sequence).add_children(children).id();

    let first = children.first().copied();
    let last = children.last().copied();

    // spawn begin observer
    let mut observer = Observer::new(
        move |trigger: Trigger<BeginEvent>,
              mut q: Query<&mut FragmentState>,
              mut commands: Commands| {
            if let Ok(mut state) = q.get_mut(id) {
                let first = first.is_some_and(|f| f == trigger.entity());
                state.active_events.insert(trigger.id.event);

                let kind = if first && trigger.kind == BeginKind::Start {
                    state.triggered += 1;
                    BeginKind::Start
                } else {
                    BeginKind::Visit
                };

                commands.trigger_targets(
                    BeginEvent {
                        id: trigger.id,
                        kind,
                    },
                    id,
                );

                info!("observed begin event! {trigger:?}");
            }
        },
    );

    for child in children {
        observer.watch_entity(*child);
    }

    commands.spawn(observer);

    // spawn end observer
    let mut observer = Observer::new(
        move |trigger: Trigger<EndEvent>,
              mut q: Query<&mut FragmentState>,
              mut commands: Commands| {
            if let Ok(mut state) = q.get_mut(id) {
                if state.active_events.remove(trigger.id.event) {
                    let last = last.is_some_and(|f| f == trigger.entity());
                    let kind = if last && trigger.kind == EndKind::End {
                        state.completed += 1;
                        EndKind::End
                    } else {
                        EndKind::Visit
                    };

                    commands.trigger_targets(
                        EndEvent {
                            id: trigger.id,
                            kind,
                        },
                        id,
                    );
                }

                info!("observed end event! {trigger:?}");
            }
        },
    );

    for child in children {
        observer.watch_entity(*child);
    }

    commands.spawn(observer);

    FragmentId::new(id)
}

macro_rules! seq_frag {
    ($count:literal, $($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<Context, Data, $($ty),*> IntoFragment<Context, Data> for ($($ty,)*)
        where
            Data: Threaded,
            Context: Threaded,
            $($ty: IntoFragment<Context, Data>),*
        {
            #[allow(unused)]
            fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
                let ($($ty,)*) = self;

                let children: [_; $count] = [
                    $($ty.into_fragment(context, commands).0),*
                ];

                spawn_sequence(&children, commands)
            }
        }
    };
}

all_tuples_with_size!(seq_frag, 0, 15, T);

fn test() -> impl IntoFragment<(), String> {
    ("Hello, world!", "How are you?")
}

impl IntoFragment<(), String> for &'static str {
    fn into_fragment(self, context: &(), commands: &mut Commands) -> FragmentId {
        <_ as IntoFragment<(), String>>::into_fragment(DataLeaf::new(self), context, commands)
    }
}

/// Recursively walk the tree depth-first, building
/// up evaluations we go.
fn descend_tree(
    node: Entity,
    evaluation: Evaluation,
    fragments: &Query<(&Evaluation, Option<&Children>, Option<&Leaf>)>,
    leaves: &mut Vec<(Entity, Evaluation)>,
) {
    let Ok((eval, children, leaf)) = fragments.get(node) else {
        return;
    };

    let new_eval = *eval & evaluation;

    info!("eval: {new_eval:?}, children: {children:?}");

    if new_eval.result.unwrap_or_default() {
        if leaf.is_some() {
            leaves.push((node, new_eval));
        } else {
            for child in children.iter().flat_map(|c| c.iter()) {
                descend_tree(*child, new_eval, fragments, leaves);
            }
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct SelectedFragments(pub Vec<Entity>);

fn select_fragments(
    roots: Query<(Entity, &Evaluation), With<Root>>,
    fragments: Query<(&Evaluation, Option<&Children>, Option<&Leaf>)>,
    f: Query<(&Evaluation, &FragmentState)>,
    mut selected_fragments: ResMut<SelectedFragments>,
) {
    // traverse trees to build up full evaluatinos
    let mut leaves = Vec::new();

    for (root, eval) in roots.iter() {
        descend_tree(root, *eval, &fragments, &mut leaves);
    }

    leaves.sort_by_key(|(_, e)| e.count);

    selected_fragments.0.clear();

    let evals: Vec<_> = f.iter().collect();
    info!("{evals:#?}");
    info!("leaves: {:?}", leaves);

    if let Some((_, eval)) = leaves.first() {
        selected_fragments.0.extend(
            leaves
                .iter()
                .take_while(|e| e.1.count == eval.count)
                .map(|(e, _)| *e),
        );
    }
}

/// A wrapper fragment that limits its children to a certain number of executions.
pub struct Limit<T> {
    fragment: T,
    limit: usize,
}

impl<T> Limit<T> {
    pub fn new(fragment: T, limit: usize) -> Self {
        Self { fragment, limit }
    }
}

#[derive(Debug, Component)]
pub struct LimitItem(usize);

impl<T, C, D> IntoFragment<C, D> for Limit<T>
where
    T: IntoFragment<C, D>,
    D: Threaded,
{
    fn into_fragment(self, context: &C, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        commands.entity(id.0).insert(LimitItem(self.limit));

        id
    }
}

fn evaluate_limits(mut fragments: Query<(&mut Evaluation, &FragmentState, &LimitItem)>) {
    for (mut eval, state, limit) in fragments.iter_mut() {
        if state.completed >= limit.0 {
            eval.merge(false.evaluate());
        }
    }
}

pub struct Evaluated<F, T, O, M> {
    pub(super) fragment: F,
    pub(super) evaluation: T,
    pub(super) _marker: PhantomData<fn() -> (O, M)>,
}

struct EvalSystem(SystemId);

// Here we automatically clean up the system when this component is removed.
impl Component for EvalSystem {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let eval = world.get::<EvalSystem>(entity).unwrap().0;
            world.commands().unregister_system(eval);
        });
    }
}

impl<Context, Data, F, T, O, M> IntoFragment<Context, Data> for Evaluated<F, T, O, M>
where
    F: IntoFragment<Context, Data>,
    T: IntoSystem<In<FragmentId>, O, M> + 'static,
    O: Evaluate + 'static,
    Data: Threaded,
{
    fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        let system = commands.register_system((move || id).pipe(self.evaluation).pipe(
            move |input: In<O>, mut evals: Query<&mut Evaluation>| {
                if let Ok(mut eval) = evals.get_mut(id.0) {
                    eval.merge(input.0.evaluate());
                }
            },
        ));

        commands.entity(id.0).insert(EvalSystem(system));

        id
    }
}

fn custom_evals(systems: Query<&EvalSystem>, mut commands: Commands) {
    for system in systems.iter() {
        commands.run_system(system.0);
    }
}
