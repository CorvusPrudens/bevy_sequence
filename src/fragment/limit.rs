use super::{FragmentNode, IntoFragment, Threaded};
use crate::{
    evaluate::{EvaluatedFragments, FragmentStates},
    FragmentId,
};
use bevy::prelude::*;

#[derive(Debug, Component)]
pub struct LimitItem {
    id: FragmentId,
    limit: usize,
}

pub fn update_limit_items(
    q: Query<&LimitItem>,
    state: Res<FragmentStates>,
    mut evals: ResMut<EvaluatedFragments>,
) {
    for LimitItem { id, limit } in q.iter() {
        let eval = state
            .state
            .get(id)
            .map(|c| c.completed < *limit)
            .unwrap_or(true);

        evals.insert(*id, eval);
    }
}

pub struct Limit<F> {
    fragment: F,
    limit: usize,
}

impl<F> Limit<F> {
    pub fn new(fragment: F, limit: usize) -> Self {
        Self { fragment, limit }
    }
}

impl<Context, Data, F> IntoFragment<Context, Data> for Limit<F>
where
    F: IntoFragment<Context, Data>,
    Data: Threaded,
{
    type Fragment = F::Fragment;

    fn into_fragment(
        self,
        context: &Context,
        commands: &mut Commands,
    ) -> (Self::Fragment, FragmentNode) {
        let (fragment, node) = self.fragment.into_fragment(context, commands);
        commands.spawn(LimitItem {
            id: node.id,
            limit: self.limit,
        });

        (fragment, node)
    }
}
