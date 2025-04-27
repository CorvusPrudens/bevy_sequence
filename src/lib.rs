//! A concise, expressive event sequencing library for Bevy.
//!
//! `bevy_sequence` is concise because you can define
//! sequences with minimal syntax.
//!
//! ```
//! (
//!     "Hello, Alice!",
//!     "Hey Bob...",
//!     "Mighty fine weather we're having, eh?",
//! )
//! ```
//!
//! It's also expressive because of its
//! rich set of combinators.
//!
//! ```
//! (
//!     // Play a sound
//!     "Hello, Alice!".sound("hello.ogg"),
//!     // Randomly select a fragment.
//!     choice(("Hey Bob...", "Aren't you supposed to be working, Bob?")),
//!     // Compute the value when this fragment is reached
//!     compute(|res: Res<Temperature>| format!("{res} degrees, huh? Mighty fine weather!")),
//! )
//!     // Run this sequence to completion just once.
//!     .once()
//! ```
//!
//! With a little setup, sequences of heterogenous types know
//! how to spawn themselves.
//!
//! ```
//! struct MyData(Cow<'static, str>);
//!
//! fn system(mut commands: Commands) {
//!     let sequence = (
//!         "Hello, Alice!",
//!         compute(|res: Res<PlayerName>| format!("Hey, {res}...")),
//!         "Mighty fine weather we're having, eh?",
//!     );
//!
//!     spawn_sequence::<MyData>(sequence, &mut commands);
//! }
//! ```

#![allow(clippy::type_complexity)]

pub mod app;
pub mod combinators;
pub mod evaluate;
pub mod fragment;

pub use crate::app::{SequencePlugin, SequenceSets};

pub mod prelude {
    pub use crate::{SequencePlugin, SequenceSets};

    pub use crate::evaluate::{Evaluate, Evaluation};

    pub use crate::fragment::{
        spawn_root, spawn_root_with, Context, Fragment, FragmentId, FragmentState, IntoFragment,
    };

    pub use crate::fragment::event::{EventId, FragmentEndEvent, FragmentEvent, IdPair};

    pub use crate::combinators::{
        //distribution::{choice, distribution},
        select::select,
        FragmentExt,
    };

    pub use crate::Threaded;
}

/// Shorthand for `Send + Sync + 'static`
pub trait Threaded: Send + Sync + 'static {}
impl<T> Threaded for T where T: Send + Sync + 'static {}
