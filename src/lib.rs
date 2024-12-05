//! A concise, expressive event sequencing library for Bevy.
//!
//! `bevy_sequence` is _concise_ because you can define
//! sequences with minimal syntax.
//!
//! ```
//! fn sequence<C, D>() -> impl IntoFragment<C, D> {
//!     (
//!         "Hello, Alice!",
//!         "Hey Bob...",
//!         "Mighty fine weather we're having, eh?",
//!     )
//! }
//! ```
//!
//! `bevy_sequence` is _expressive_ because of its
//! rich set of combinators.
//!
//! ```
//! # fn rand_float() -> f32 { 4. }
//! fn sequence<C, D>() -> impl IntoFragment<C, D> {
//!     (
//!         "Hello, Alice!",
//!         any((
//!             "Hey Bob...",
//!             "Aren't you supposed to be working?".eval(|res: Res<Branch>| res.0 < 0.25),
//!         ))
//!         .on_start(|res: ResMut<Branch>| res.0 = rand_float()),
//!         "Mighty fine weather we're having, eh?",
//!     )
//!     .once()
//!     .set_resource(ConversationStarted(true))
//! }
//! ```

pub mod app;
pub mod evaluate;
pub mod fragment;

pub mod prelude {
    pub use crate::app::{SequencePlugin, SequenceSets};

    pub use crate::evaluate::{Evaluate, Evaluation};

    pub use crate::fragment::{
        EventId, Fragment, FragmentEndEvent, FragmentEvent, FragmentId, FragmentState, IdPair,
        IntoFragment,
    };

    pub use crate::Threaded;
}

/// Shorthand for `Send + Sync + 'static`
pub trait Threaded: Send + Sync + 'static {}
impl<T> Threaded for T where T: Send + Sync + 'static {}
