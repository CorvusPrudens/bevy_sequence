use super::{End, Fragment, FragmentNode, IntoFragment, Start, Threaded};
use crate::evaluate::{EvaluatedFragments, FragmentStates};
use crate::{FragmentEvent, FragmentId};
use bevy::prelude::*;
use bevy::utils::all_tuples_with_size;

#[derive(Debug, Component)]
pub struct SequenceItems {
    id: FragmentId,
    children: Vec<FragmentId>,
}

pub fn update_sequence_items(
    q: Query<&SequenceItems>,
    state: Res<FragmentStates>,
    mut evals: ResMut<EvaluatedFragments>,
) {
    for SequenceItems { id, children } in q.iter() {
        let outer_finished = state.state.get(id).map(|s| s.completed).unwrap_or_default();

        // look for the first item that has finished equal to the container
        let mut first_selected = false;
        for child in children.iter() {
            if !first_selected {
                let inner_finished = state
                    .state
                    .get(child)
                    .map(|s| s.completed)
                    .unwrap_or_default();

                if inner_finished <= outer_finished {
                    first_selected = true;
                    evals.insert(*child, true);

                    continue;
                }
            }

            evals.insert(*child, false);
        }
    }
}

pub struct Sequence<F> {
    fragments: F,
    id: FragmentId,
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
            type Fragment = Sequence<($($ty::Fragment,)*)>;

            #[allow(unused)]
            fn into_fragment(self, context: &Context, commands: &mut Commands) -> (Self::Fragment, FragmentNode) {
                let id = todo!();
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
                let ($($ty,)*) = self;

                let seq = Sequence {
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
                };

                // TODO: this is basically a leak. It would be nice if we could
                // remove this when this sequene is no longer active.
                commands.spawn(SequenceItems { id: seq.id, children: ids });

                (seq, node)
            }
        }

        #[allow(non_snake_case)]
        impl<Context, Data, $($ty),*> Fragment<Context, Data> for Sequence<($($ty,)*)>
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

                if states.first().is_some_and(|f| f.entered()) {
                    state.update(self.id).triggered += 1;
                    Start::Entered
                } else if states.iter().any(|f| f.visited()) {
                    Start::Visited
                } else {
                    Start::Unvisited
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

                if states.last().is_some_and(|f| f.exited()) {
                    state.update(self.id).completed += 1;
                    End::Exited
                } else if states.iter().any(|f| f.visited()) {
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

all_tuples_with_size!(seq_frag, 0, 15, T);

impl<Context, Data, T> IntoFragment<Context, Data> for Vec<T>
where
    Data: Threaded,
    T: IntoFragment<Context, Data>,
{
    type Fragment = Sequence<Vec<T::Fragment>>;

    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> (Self::Fragment, FragmentNode) {
        let id = todo!();
        let mut ids = Vec::new();
        let mut node = FragmentNode::new(id, Vec::new());

        let fragments = self
            .into_iter()
            .map(|frag| {
                let (frag, n) = frag.into_fragment(context, commands);
                ids.push(n.id);
                node.push(n);
                frag
            })
            .collect();

        let seq = Sequence { fragments, id };

        commands.spawn(SequenceItems {
            id: seq.id,
            children: ids,
        });

        (seq, node)
    }
}

impl<Context, Data, T, const LEN: usize> IntoFragment<Context, Data> for [T; LEN]
where
    Data: Threaded,
    T: IntoFragment<Context, Data>,
{
    type Fragment = Sequence<[T::Fragment; LEN]>;

    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> (Self::Fragment, FragmentNode) {
        let id = todo!();
        let mut ids = Vec::new();
        let mut node = FragmentNode::new(id, Vec::new());

        let mut fragments = self.into_iter();
        let fragments = core::array::from_fn(|_| {
            let (frag, n) = fragments.next().unwrap().into_fragment(context, commands);
            ids.push(n.id);
            node.push(n);
            frag
        });

        let seq = Sequence { fragments, id };

        commands.spawn(SequenceItems {
            id: seq.id,
            children: ids,
        });

        (seq, node)
    }
}

macro_rules! impl_iterable {
    ($ty:ident, $col:ty) => {
        impl<Context, Data, $ty> Fragment<Context, Data> for Sequence<$col>
        where
            Data: Threaded,
            $ty: Fragment<Context, Data>,
        {
            fn start(
                &mut self,
                context: &Context,
                id: FragmentId,
                state: &mut FragmentStates,
                writer: &mut EventWriter<FragmentEvent<Data>>,
                commands: &mut Commands,
            ) -> Start {
                let mut start = Start::Unvisited;

                for (i, frag) in self.fragments.iter_mut().enumerate() {
                    let frag_start = frag.start(context, id, state, writer, commands);

                    if i == 0 && frag_start.entered() {
                        state.update(self.id).triggered += 1;
                        start = Start::Entered;
                    }

                    if frag_start.visited() && start == Start::Unvisited {
                        start = Start::Visited;
                    }
                }

                start
            }

            fn end(
                &mut self,
                context: &Context,
                id: FragmentId,
                state: &mut FragmentStates,
                commands: &mut Commands,
            ) -> End {
                let mut end = End::Unvisited;
                let len = self.fragments.len();

                for (i, frag) in self.fragments.iter_mut().enumerate() {
                    let frag_end = frag.end(context, id, state, commands);

                    if i == len - 1 && frag_end.exited() {
                        state.update(self.id).completed += 1;
                        end = End::Exited;
                    }

                    if frag_end.visited() && end == End::Unvisited {
                        end = End::Visited;
                    }
                }

                end
            }

            fn id(&self) -> &FragmentId {
                &self.id
            }
        }
    };
}

impl_iterable!(T, Vec<T>);

// TODO: consolidate this somehow
impl<Context, Data, T, const LEN: usize> Fragment<Context, Data> for Sequence<[T; LEN]>
where
    Data: Threaded,
    T: Fragment<Context, Data>,
{
    fn start(
        &mut self,
        context: &Context,
        id: FragmentId,
        state: &mut FragmentStates,
        writer: &mut EventWriter<FragmentEvent<Data>>,
        commands: &mut Commands,
    ) -> Start {
        let mut start = Start::Unvisited;

        for (i, frag) in self.fragments.iter_mut().enumerate() {
            let frag_start = frag.start(context, id, state, writer, commands);

            if i == 0 && frag_start.entered() {
                state.update(self.id).triggered += 1;
                start = Start::Entered;
            }

            if frag_start.visited() && start == Start::Unvisited {
                start = Start::Visited;
            }
        }

        start
    }

    fn end(
        &mut self,
        context: &Context,
        id: FragmentId,
        state: &mut FragmentStates,
        commands: &mut Commands,
    ) -> End {
        let mut end = End::Unvisited;
        let len = self.fragments.len();

        for (i, frag) in self.fragments.iter_mut().enumerate() {
            let frag_end = frag.end(context, id, state, commands);

            if i == len - 1 && frag_end.exited() {
                state.update(self.id).completed += 1;
                end = End::Exited;
            }

            if frag_end.visited() && end == End::Unvisited {
                end = End::Visited;
            }
        }

        end
    }

    fn id(&self) -> &FragmentId {
        &self.id
    }
}
