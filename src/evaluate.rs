use crate::FragmentId;
use bevy_ecs::prelude::*;
use std::collections::{hash_map::Entry, HashMap};

pub trait Evaluate: sealed::Sealed {
    fn evaluate(&self) -> Evaluation;
}

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Evaluation {}
    impl<const LEN: usize> Sealed for [bool; LEN] {}
    impl Sealed for Vec<bool> {}
    impl Sealed for bool {}
}

impl Evaluate for bool {
    fn evaluate(&self) -> Evaluation {
        Evaluation {
            result: Some(*self),
            count: 1,
        }
    }
}

impl<const LEN: usize> Evaluate for [bool; LEN] {
    fn evaluate(&self) -> Evaluation {
        let result = if LEN == 0 {
            None
        } else {
            Some(self.iter().all(|e| *e))
        };

        Evaluation {
            result,
            count: self.len(),
        }
    }
}

impl Evaluate for Vec<bool> {
    fn evaluate(&self) -> Evaluation {
        let result = if self.len() == 0 {
            None
        } else {
            Some(self.iter().all(|e| *e))
        };

        Evaluation {
            result,
            count: self.len(),
        }
    }
}

impl Evaluate for Evaluation {
    fn evaluate(&self) -> Evaluation {
        *self
    }
}

#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Evaluation {
    pub result: Option<bool>,
    pub count: usize,
}

impl Evaluation {
    pub fn merge(&mut self, other: Evaluation) {
        *self = *self & other;
    }
}

impl core::ops::BitAnd for Evaluation {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self.result, rhs.result) {
            (Some(a), Some(b)) => Self {
                result: Some(a && b),
                count: self.count + rhs.count,
            },
            (None, Some(_)) => rhs,
            (Some(_), None) | (None, None) => self,
        }
    }
}

#[derive(Debug, Component, Default, Clone, Copy, PartialEq, Eq)]
pub struct FragmentState {
    pub triggered: usize,
    pub completed: usize,
    pub active: bool,
}

#[derive(Resource, Debug, Default)]
pub struct FragmentStates {
    pub state: HashMap<FragmentId, FragmentState>,
}

#[allow(unused)]
impl FragmentStates {
    pub fn update(&mut self, id: FragmentId) -> &mut FragmentState {
        self.state.entry(id).or_default()
    }

    pub fn get(&self, id: FragmentId) -> Option<&FragmentState> {
        self.state.get(&id)
    }

    pub fn is_done(&self, id: FragmentId) -> bool {
        self.state
            .get(&id)
            .is_some_and(|s| s.completed >= 1 && !s.active)
    }

    pub fn is_active(&self, id: FragmentId) -> bool {
        self.state.get(&id).is_some_and(|s| s.active)
    }

    pub fn has_triggered(&self, id: FragmentId) -> bool {
        self.state.get(&id).is_some_and(|s| s.triggered > 0)
    }
}

#[derive(Resource, Debug, Default)]
pub struct EvaluatedFragments {
    pub(super) evaluations: HashMap<FragmentId, Evaluation>,
}

#[allow(unused)]
impl EvaluatedFragments {
    pub fn insert<E: Evaluate>(&mut self, id: FragmentId, evaluation: E) {
        let eval = evaluation.evaluate();
        match self.evaluations.entry(id) {
            Entry::Vacant(e) => {
                e.insert(eval);
            }
            Entry::Occupied(mut e) => e.get_mut().merge(eval),
        }
    }

    pub fn get(&self, id: FragmentId) -> Option<Evaluation> {
        self.evaluations.get(&id).copied()
    }

    /// Returns whether the provided ID should be further evaulated.
    ///
    /// An ID not in the set will always return false.
    pub fn is_candidate(&self, id: FragmentId) -> bool {
        self.evaluations
            .get(&id)
            .and_then(|e| e.result)
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.evaluations.clear();
    }
}
