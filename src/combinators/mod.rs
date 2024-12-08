use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub mod evaluated;
pub mod limit;
pub mod select;
pub mod sequence;

pub use evaluated::{Evaluated, EvaluatedWithId};
pub use limit::Limit;
pub use sequence::Sequence;

pub struct CombinatorPlugin;

impl Plugin for CombinatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                sequence::update_sequence_items,
                evaluated::custom_evals,
                evaluated::custom_evals_ids,
                limit::evaluate_limits,
                select::update_select_items,
            )
                .in_set(crate::app::SequenceSets::Evaluate),
        );
        // .add_observer(sequence::sequence_begin_observer)
        // .add_observer(sequence::sequence_end_observer);
    }
}
