use crate::prelude::*;
use bevy_ecs::prelude::*;

// Update the trait to remove the const generic and return an iterator
pub trait IntoChildren<Data: Threaded, C = ()> {
    type Collection: AsRef<[Entity]>;

    fn into_children(self, context: &Context<C>, commands: &mut Commands) -> Self::Collection;
}

macro_rules! children_frag {
    ($count:literal, $($name:ident),*) => {
        #[allow(non_snake_case, unused_variables)]
        impl<Data, C, $($name),*> IntoChildren<Data, C> for ($($name,)*)
        where
            Data: Threaded,
            $($name: IntoFragment<Data, C>),*
        {
            type Collection = [Entity; $count];

            fn into_children(self, context: &Context<C>, commands: &mut Commands) -> Self::Collection {
                let ($($name,)*) = self;
                let entities: [Entity; $count] = [$($name.into_fragment(context, commands).entity()),*];
                entities
            }
        }
    }
}

variadics_please::all_tuples_with_size!(children_frag, 0, 15, T);

// Update the `IntoChildren` trait to support arrays and vectors.
// For arrays (with a const generic length) and for vectors of fragments.
impl<C, Data, T, const N: usize> IntoChildren<Data, C> for [T; N]
where
    Data: Threaded,
    T: IntoFragment<Data, C>,
{
    type Collection = [Entity; N];

    fn into_children(self, context: &Context<C>, commands: &mut Commands) -> Self::Collection {
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

impl<Data, C, T> IntoChildren<Data, C> for Vec<T>
where
    Data: Threaded,
    T: IntoFragment<Data, C>,
{
    type Collection = Vec<Entity>;

    fn into_children(self, context: &Context<C>, commands: &mut Commands) -> Self::Collection {
        let entities: Vec<Entity> = self
            .into_iter()
            .map(|f| f.into_fragment(context, commands).entity())
            .collect();

        entities
    }
}
