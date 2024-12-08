use crate::fragment::event::{BeginStage, EndStage, MapContext, MapFn, StageEvent};
use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;

#[derive(Component)]
#[require(Fragment)]
pub struct Sequence;

pub(super) fn update_sequence_items(
    q: Query<(&Children, &FragmentState), With<Sequence>>,
    mut children: Query<(&mut Evaluation, &FragmentState)>,
) {
    for (seq, outer_state) in q.iter() {
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

fn map_begin(input: MapContext<BeginStage>, first: Option<Entity>) -> StageEvent<BeginStage> {
    let first = match (first, input.child) {
        (Some(first), Some(child)) => first == child,
        _ => false,
    };

    if first && input.event.stage == BeginStage::Start {
        StageEvent {
            id: input.event.id,
            stage: BeginStage::Start,
        }
    } else {
        StageEvent {
            id: input.event.id,
            stage: BeginStage::Visit,
        }
    }
}

fn map_end(input: MapContext<EndStage>, last: Option<Entity>) -> StageEvent<EndStage> {
    let last = match (last, input.child) {
        (Some(last), Some(child)) => last == child,
        _ => false,
    };

    if last && input.event.stage == EndStage::End {
        StageEvent {
            id: input.event.id,
            stage: EndStage::End,
        }
    } else {
        StageEvent {
            id: input.event.id,
            stage: EndStage::Visit,
        }
    }
}

macro_rules! seq_frag {
    ($count:literal, $($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<Data, Context, $($ty),*> IntoFragment<Data, Context> for ($($ty,)*)
        where
            Data: Threaded,
            $($ty: IntoFragment<Data, Context>),*
        {
            #[allow(unused)]
            fn into_fragment(self, context: &Context, commands: &mut Commands) -> FragmentId {
                let ($($ty,)*) = self;

                let children: [_; $count] = [
                    $($ty.into_fragment(context, commands).entity()),*
                ];

                let first = children.first().copied();
                let last = children.last().copied();

                let map_begin = MapFn::function(move |input| map_begin(input, first));
                let map_end = MapFn::function(move |input| map_end(input, last));
                FragmentId::new(commands.spawn((Sequence, map_begin, map_end)).add_children(&children).id())
            }
        }
    };
}

bevy_utils::all_tuples_with_size!(seq_frag, 0, 15, T);
