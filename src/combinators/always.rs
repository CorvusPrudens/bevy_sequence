use crate::prelude::*;
use bevy_ecs::prelude::*;

/// Always insert a true evaluation.
///
/// This does not necessarily mean that the fragment will always run;
/// any false evaluation will still cause this fragment to be skipped.
pub struct AlwaysFragment<T> {
    fragment: T,
}

impl<T> AlwaysFragment<T> {
    pub fn new(fragment: T) -> Self {
        Self { fragment }
    }
}

#[derive(Debug, Component)]
pub struct Always;

impl<T, C, D> IntoFragment<D, C> for AlwaysFragment<T>
where
    T: IntoFragment<D, C>,
    D: Threaded,
{
    fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
        let id = self.fragment.into_fragment(context, commands);
        commands.entity(id.entity()).insert(Always);

        id
    }
}

pub(super) fn evaluate_always(mut fragments: Query<&mut Evaluation, With<Always>>) {
    for mut eval in fragments.iter_mut() {
        eval.merge(true.evaluate());
    }
}
