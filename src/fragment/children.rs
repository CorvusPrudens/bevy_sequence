use crate::prelude::*;
use bevy_ecs::prelude::*;

// Update the trait to remove the const generic and return an iterator
pub trait IntoChildren<Context, Data: Threaded> {
    type Collection: AsRef<[Entity]>;

    fn into_children(self, context: &Context, commands: &mut Commands) -> Self::Collection;
}

macro_rules! children_frag {
    ($count:literal, $($name:ident),*) => {
        #[allow(non_snake_case, unused_variables)]
        impl<Context, Data, $($name),*> IntoChildren<Context, Data> for ($($name,)*)
        where
            Data: Threaded,
            Context: Threaded,
            $($name: IntoFragment<Context, Data>),*
        {
            fn into_children(self, context: &Context, commands: &mut Commands) -> impl AsRef<[Entity]> {
                let ($($name,)*) = self;
                let entities: [Entity; $count] = [$($name.into_fragment(context, commands).entity()),*];
                entities
            }
        }
    }
}

bevy_utils::all_tuples_with_size!(children_frag, 0, 15, T);

// Update the `IntoChildren` trait to support arrays and vectors.
// For arrays (with a const generic length) and for vectors of fragments.
impl<Context, Data, T, const N: usize> IntoChildren<Context, Data> for [T; N]
where
    Data: Threaded,
    Context: Threaded,
    T: IntoFragment<Context, Data>,
{
    fn into_children(self, context: &Context, commands: &mut Commands) -> impl AsRef<[Entity]> {
        let mut entities = self.into_iter();
        let entities: [_; N] = std::array::from_fn(|_| {
            entities
                .next()
                .unwrap()
                .into_fragment(context, commands)
                .entity()
        });

        entities
    }
}

impl<Context, Data, T> IntoChildren<Context, Data> for Vec<T>
where
    Data: Threaded,
    Context: Threaded,
    T: IntoFragment<Context, Data>,
{
    fn into_children(self, context: &Context, commands: &mut Commands) -> impl AsRef<[Entity]> {
        let entities: Vec<Entity> = self
            .into_iter()
            .map(|f| f.into_fragment(context, commands).entity())
            .collect();

        entities
    }
}
