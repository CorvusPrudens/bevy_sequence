use crate::{app::AddSystemsChecked, fragment::children::IntoChildren, prelude::*};
use bevy_app::PreUpdate;
use bevy_ecs::prelude::*;
use rand::distributions::{WeightedIndex, uniform::SampleUniform};

/// A fragment that randomly selects its children.
///
/// Equivalent to [DistributionFragment] where all weights are equal.
pub struct ChoiceFragment<F> {
    fragments: F,
}

/// A fragment that randomly selects its children.
///
/// Equivalent to [DistributionFragment] where all weights are equal.
pub fn choice<F>(fragments: F) -> ChoiceFragment<F> {
    ChoiceFragment { fragments }
}

pub fn test() {}

impl<D, C, F> IntoFragment<D, C> for ChoiceFragment<F>
where
    D: Threaded,
    F: IntoChildren<D, C>,
{
    fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
        let children = self.fragments.into_children(context, commands);
        commands.add_systems_checked(PreUpdate, test.in_set(SequenceSets::Evaluate));

        let mut entity = commands.spawn((Fragment, DistributionActiveNode(0)));

        entity.add_children(children.as_ref());

        match WeightedIndex::new(children.as_ref().iter().map(|_| 1)) {
            Ok(distribution) => {
                entity.insert(Distribution(distribution));
            }
            Err(e) => {
                bevy_log::error!("unable to spawn choice fragment: {e}");
            }
        }

        FragmentId::new(entity.id())
    }
}

/// A fragment that selects its children based on a probability distribution.
pub struct DistributionFragment<F, D, const LENGTH: usize> {
    fragments: F,
    distribution: [D; LENGTH],
}

/// A fragment that selects its children based on a probability distribution.
pub fn distribution<F, D, const LENGTH: usize>(
    fragments: F,
    distribution: [D; LENGTH],
) -> DistributionFragment<F, D, LENGTH> {
    DistributionFragment {
        fragments,
        distribution,
    }
}

#[derive(Clone, Copy, Component)]
#[require(Fragment)]
pub(super) struct DistributionActiveNode(usize);

#[derive(Component)]
pub(super) struct Distribution<X: SampleUniform + PartialOrd>(WeightedIndex<X>);

macro_rules! distribution_implementation {
    ($count:literal, $($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<C, Data, D, $($ty),*> IntoFragment<Data, C> for DistributionFragment<($($ty,)*), D, $count>
        where
            Data: Threaded,
            D: SampleUniform + PartialOrd + for<'a> std::ops::AddAssign<&'a D> + Clone + Default + Threaded,
            D::Sampler: Threaded,
            $($ty: IntoFragment<Data, C>),*
        {
            fn into_fragment(self, context: &Context<C>, commands: &mut Commands) -> FragmentId {
                let ($($ty,)*) = self.fragments;
                let children = [$($ty.into_fragment(context, commands) .entity()),*];
                commands.add_systems_checked(PreUpdate, update_distribution_items::<D>.in_set(SequenceSets::Evaluate));

                let mut entity = commands
                    .spawn(DistributionActiveNode(0));

                entity.add_children(&children);

                match WeightedIndex::new(self.distribution){
                    Ok(distribution)=> {
                        entity.insert(Distribution(distribution));
                    }
                    Err(e)=> {
                        bevy_log::error!("unable to spawn distribution fragment: {e}");
                    }
                }

                FragmentId::new(entity.id())
            }
        }
    };
}

variadics_please::all_tuples_with_size!(distribution_implementation, 1, 15, T);

pub(super) fn update_distribution_items<X>(
    mut choices: Query<(
        &Children,
        &FragmentState,
        &Distribution<X>,
        &mut DistributionActiveNode,
    )>,
    mut children_query: Query<&mut Evaluation>,
) where
    X: SampleUniform + PartialOrd + Threaded,
    X::Sampler: Threaded,
{
    use rand::prelude::*;

    let mut rng = rand::thread_rng();

    for (children, state, distribution, mut active) in choices.iter_mut() {
        let selection = if !state.active {
            let selection = distribution.0.sample(&mut rng);
            active.0 = selection;
            selection
        } else {
            active.0
        };

        for (i, child) in children.iter().enumerate() {
            let Ok(mut evaluation) = children_query.get_mut(child) else {
                continue;
            };
            evaluation.merge((selection == i).evaluate());
        }
    }
}
