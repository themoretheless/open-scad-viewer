//! Run the actual private denominator proof in isolation from legacy codec-only
//! unit modules, including in a genuinely codec-free library dependency graph.
//! The failed whole-lib no-codec test build remains a separate recorded debt.
use num_rational::BigRational as Q;
use nurbs_core::{curve::Curve, editing::exact_knot_insertion as curve_edit};
mod rational_factor {
    pub use nurbs_core::editing::rational_factor::Refusal;
}
#[allow(dead_code)]
#[path = "../src/editing/rational_edit.rs"]
mod rational_edit;
use rational_edit::Work;
type PResult<T> = std::result::Result<T, curve_edit::Refusal>;
#[path = "../src/editing/exact_surface_knot_insertion/denominator.rs"]
mod denominator;
