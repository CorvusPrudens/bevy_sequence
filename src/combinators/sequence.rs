use crate::fragment::{BeginEvent, BeginKind, EndEvent, EndKind};
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

pub(super) fn sequence_begin_observer(
    trigger: Trigger<BeginEvent>,
    mut parent: Query<(&mut FragmentState, &Children), With<Sequence>>,
    child: Query<&Parent>,
    mut commands: Commands,
) {
    let child_id = trigger.entity();
    let Ok(parent_id) = child.get(child_id).map(|p| p.get()) else {
        return;
    };

    let Ok((mut state, children)) = parent.get_mut(parent_id) else {
        return;
    };

    let first = children.first().is_some_and(|f| *f == child_id);
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
        parent_id,
    );

    // info!("observed begin event! {trigger:?}");
}

pub(super) fn sequence_end_observer(
    trigger: Trigger<EndEvent>,
    mut parent: Query<(&mut FragmentState, &Children), With<Sequence>>,
    child: Query<&Parent>,
    mut commands: Commands,
) {
    let child_id = trigger.entity();
    let Ok(parent_id) = child.get(child_id).map(|p| p.get()) else {
        return;
    };

    let Ok((mut state, children)) = parent.get_mut(parent_id) else {
        return;
    };

    let last = children.last().is_some_and(|f| *f == child_id);

    if state.active_events.remove(trigger.id.event) {
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
            parent_id,
        );
    }

    // info!("observed end event! {trigger:?}");
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
                    $($ty.into_fragment(context, commands).entity()),*
                ];

                FragmentId::new(commands.spawn(Sequence).add_children(&children).id())
            }
        }
    };
}

bevy_utils::all_tuples_with_size!(seq_frag, 0, 15, T);
