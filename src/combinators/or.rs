use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::BuildChildren;

/// Evaluate true for B when A is false.
pub struct Or<A, B> {
    a: A,
    b: B,
}

impl<A, B> Or<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

#[derive(Debug, Component)]
pub struct OrItem;

impl<A, B, C, D> IntoFragment<D, C> for Or<A, B>
where
    A: IntoFragment<D, C>,
    B: IntoFragment<D, C>,
    D: Threaded,
{
    fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
        let a = self.a.into_fragment(context, commands);
        let b = self.b.into_fragment(context, commands);

        commands.entity(b.entity()).insert(OrItem);

        FragmentId::new(
            commands
                .spawn(Fragment)
                .add_children(&[a.entity(), b.entity()])
                .id(),
        )
    }
}
