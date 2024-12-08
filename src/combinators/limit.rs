use crate::prelude::*;
use bevy_ecs::prelude::*;

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

impl<T, C, D> IntoFragment<D, C> for Limit<T>
where
    T: IntoFragment<D, C>,
    D: Threaded,
{
    fn into_fragment(self, context: &C, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        commands.entity(id.entity()).insert(LimitItem(self.limit));

        id
    }
}

pub(super) fn evaluate_limits(mut fragments: Query<(&mut Evaluation, &FragmentState, &LimitItem)>) {
    for (mut eval, state, limit) in fragments.iter_mut() {
        if state.completed >= limit.0 {
            eval.merge(false.evaluate());
        }
    }
}
