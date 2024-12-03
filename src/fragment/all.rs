use super::{
    End, Fragment, FragmentEvent, FragmentId, FragmentNode, IntoFragment, Start, Threaded,
};
use crate::evaluate::{EvaluatedFragments, FragmentStates};
use bevy::{prelude::*, utils::all_tuples_with_size};

#[derive(Default, Clone)]
struct ItemState {
    entered: bool,
    exited: bool,
}

pub fn all<A>(fragments: A) -> All<A> {
    All {
        fragments,
        id: todo!(),
        states: Vec::new(),
    }
}

#[derive(Debug, Component)]
pub struct AllItems {
    id: FragmentId,
    children: Vec<FragmentId>,
}

pub struct All<A> {
    fragments: A,
    id: FragmentId,
    states: Vec<ItemState>,
}

pub fn update_all_items(
    q: Query<&AllItems>,
    state: Res<FragmentStates>,
    mut evals: ResMut<EvaluatedFragments>,
) {
    for AllItems { children, id } in q.iter() {
        let outer_finished = state.state.get(id).map(|s| s.completed).unwrap_or_default();
        for child in children {
            // All children should all be evaluated identically.

            let finished = state
                .state
                .get(child)
                .map(|s| s.completed)
                .unwrap_or_default();

            evals.insert(*child, finished <= outer_finished);
        }
    }
}

macro_rules! any_frag {
    ($count:literal, $($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<Context, Data, $($ty),*> IntoFragment<Context, Data> for All<($($ty,)*)>
        where
            Data: Threaded,
            Context: Threaded,
            $($ty: IntoFragment<Context, Data>),*
        {
            type Fragment = All<($($ty::Fragment,)*)>;

            #[allow(unused_mut, unused_variables)]
            fn into_fragment(self, context: &Context, commands: &mut Commands) -> (Self::Fragment, FragmentNode) {
                let id = self.id;
                let mut ids = Vec::new();
                ids.reserve_exact($count);
                let mut node = FragmentNode::new(
                    id,
                    {
                        let mut children = Vec::new();
                        children.reserve_exact($count);
                        children
                    }
                );
                let ($($ty,)*) = self.fragments;

                let seq = All {
                    fragments: (
                        $(
                            {
                                let (frag, n) = $ty.into_fragment(context, commands);
                                ids.push(n.id);
                                node.push(n);
                                frag
                            },
                        )*
                    ),
                    id,
                    states: vec![Default::default(); $count],
                };

                // TODO: this is basically a leak. It would be nice if we could
                // remove this when this sequene is no longer active.
                commands.spawn(AllItems { id: seq.id, children: ids });

                (seq, node)
            }
        }

        #[allow(non_snake_case)]
        impl<Context, Data, $($ty),*> Fragment<Context, Data> for All<($($ty,)*)>
        where
            Data: Threaded,
            Context: Threaded,
            $($ty: Fragment<Context, Data>),*
        {
            #[allow(unused)]
            fn start(
                &mut self,
                context: &Context,
                id: FragmentId,
                state: &mut FragmentStates,
                writer: &mut EventWriter<FragmentEvent<Data>>,
                commands: &mut Commands,
            ) -> Start {
                let ($($ty,)*) = &mut self.fragments;
                let states: [Start; $count] = [
                    $($ty.start(context, id, state, writer, commands)),*
                ];

                let unentered = self.states.iter().all(|s| !s.entered);
                let mut any_entered = false;
                let mut any_visited = false;

                for (new_state, old_state) in states.iter().zip(self.states.iter_mut()) {
                    if new_state.entered() {
                        old_state.entered = true;
                        any_entered = true;
                    }
                    if new_state.visited() {
                        any_visited = true;
                    }
                }

                match (unentered, any_entered, any_visited) {
                    (true, true, _) => {
                        state.update(self.id).triggered += 1;
                        Start::Entered
                    },
                    (false, true, false) | (false, false, true) => Start::Visited,
                    _ => Start::Unvisited,
                }
            }

            #[allow(unused)]
            fn end(
                &mut self,
                context: &Context,
                id: FragmentId,
                state: &mut FragmentStates,
                commands: &mut Commands
            ) -> End {
                let ($($ty,)*) = &mut self.fragments;
                let states: [End; $count] = [
                    $($ty.end(context, id, state, commands)),*
                ];

                let mut any_visited = false;

                for (new_state, old_state) in states.iter().zip(self.states.iter_mut()) {
                    if new_state.exited() {
                        old_state.exited = true;
                    }

                    // This will catch both
                    if new_state.visited() {
                        any_visited = true;
                    }
                }

                let all_exited = self.states.iter().all(|s| s.exited);
                if all_exited {
                    state.update(self.id).completed += 1;
                    End::Exited
                } else if any_visited {
                    End::Visited
                } else {
                    End::Unvisited
                }
            }

            fn id(&self) -> &FragmentId {
                &self.id
            }
        }
    };
}

all_tuples_with_size!(any_frag, 0, 15, T);
