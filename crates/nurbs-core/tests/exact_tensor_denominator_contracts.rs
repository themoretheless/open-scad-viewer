//! Run the actual private denominator proof in isolation from legacy codec-only
//! unit modules, including in a genuinely codec-free library dependency graph.
//! Historical failed harness builds remain preserved in qualification receipts.
use num_rational::BigRational as Q;
use nurbs_core::{curve::Curve, editing::exact_knot_insertion as curve_edit};
#[allow(dead_code)]
#[path = "../../nurbs-geometry/src/bounded_rational.rs"]
pub(crate) mod bounded_rational;
mod core {
    pub(crate) use crate::bounded_rational;
}
mod rational_factor {
    pub use nurbs_core::editing::rational_factor::Refusal;
    impl From<crate::bounded_rational::Error> for Refusal {
        fn from(e: crate::bounded_rational::Error) -> Self {
            match e {
                crate::bounded_rational::Error::ResourceLimit => Self::ResourceLimit,
                crate::bounded_rational::Error::NotRepresentable => Self::NotRepresentable,
            }
        }
    }
}
#[allow(dead_code)]
#[path = "../src/editing/rational_edit.rs"]
mod rational_edit;
use rational_edit::Work;
type PResult<T> = std::result::Result<T, curve_edit::Refusal>;
#[path = "../src/editing/exact_surface_knot_insertion/denominator.rs"]
mod denominator;
