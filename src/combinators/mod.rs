use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use std::borrow::Cow;

pub mod always;
pub mod distribution;
pub mod evaluated;
pub mod hooks;
pub mod limit;
pub mod or;
pub mod save;
pub mod select;
pub mod sequence;

pub use always::AlwaysFragment;
pub use evaluated::{Evaluated, EvaluatedWithId};
pub use hooks::{OnEnd, OnStart, OnVisit};
pub use limit::Limit;
pub use or::Or;
pub use save::Save;
pub use sequence::Sequence;

use crate::prelude::{Evaluate, FragmentId};

pub struct CombinatorPlugin;

impl Plugin for CombinatorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(save::SavedSequences::default())
            .add_systems(
                PreUpdate,
                (
                    sequence::update_sequence_items,
                    evaluated::custom_evals,
                    evaluated::custom_evals_ids,
                    limit::evaluate_limits,
                    select::update_select_items,
                    always::evaluate_always,
                )
                    .in_set(crate::app::SequenceSets::Evaluate),
            )
            .add_systems(
                PostUpdate,
                save::sync_sequence.in_set(crate::app::SequenceSets::Save),
            )
            .add_observer(save::load_sequence);
    }
}

impl<T> FragmentExt for T {}

pub trait FragmentExt: Sized {
    /// Limit this fragment to `n` triggers.
    fn limit(self, n: usize) -> Limit<Self> {
        Limit::new(self, n)
    }

    /// Set this fragment's limit to 1.
    fn once(self) -> Limit<Self> {
        self.limit(1)
    }

    /// Always insert a true evaluation.
    ///
    /// This does not necessarily mean that the fragment will always run;
    /// any false evaluation will still cause this fragment to be skipped.
    fn always(self) -> AlwaysFragment<Self> {
        AlwaysFragment::new(self)
    }

    /// If this fragment evaluates to false,
    /// add a true evaluation to the passed in fragment B.
    fn or<B>(self, fragment: B) -> Or<Self, B> {
        Or::new(self, fragment)
    }

    /// Add an evaluation to this fragment.
    fn eval<S, O, M>(self, system: S) -> Evaluated<Self, S, O, M>
    where
        S: IntoSystem<(), O, M> + 'static,
        O: Evaluate + 'static,
    {
        Evaluated::new(self, system)
    }

    /// Add an evaluation to this fragment.
    ///
    /// This will pass the fragment's ID to the provided systme.
    fn eval_id<S, O, M>(self, system: S) -> EvaluatedWithId<Self, S, O, M>
    where
        S: IntoSystem<In<FragmentId>, O, M> + 'static,
        O: Evaluate + 'static,
    {
        EvaluatedWithId::new(self, system)
    }

    /// Run a system when this fragment is first reached.
    ///
    /// [OnStart] systems will be run from top-to-bottom.
    /// ```
    /// (
    ///     "fragment".on_start(|| /* Second */),
    /// )
    ///     .on_start(|| /* First */)
    /// ```
    fn on_start<S, M>(self, system: S) -> OnStart<Self, S, M>
    where
        S: IntoSystem<(), (), M> + Send + Sync + 'static,
    {
        hooks::on_start(self, system)
    }

    /// Run a system when this fragment finishes.
    ///
    /// [OnEnd] systems will be run from bottom-to-top.
    /// ```
    /// (
    ///     "fragment".on_end(|| /* First */),
    /// )
    ///     .on_end(|| /* Second */)
    /// ```
    fn on_end<S, M>(self, system: S) -> OnEnd<Self, S, M>
    where
        S: IntoSystem<(), (), M> + Send + Sync + 'static,
    {
        hooks::on_end(self, system)
    }

    /// Synchronize this fragment's state with a [save::SavedSequence] component.
    ///
    /// Fragments with this component will automatically load any previously-saved
    /// state when spawned.
    fn save_as(self, name: impl Into<Cow<'static, str>>) -> Save<Self>
    where
        Self: 'static,
    {
        Save::new(self, name.into())
    }
}
