//! Прямое редактирование, морфинг и сглаживание.
pub mod edit;
pub mod weight_edit;
pub mod surface_edit;
pub mod surface_extension;
pub mod direct_edit;
pub mod morph;
pub mod fairing;
pub mod surface_fairing;
pub mod deform;
pub mod linear_simplification;
pub mod exact_knot_removal;
pub mod exact_surface_knot_removal;
pub mod exact_degree_reduction;
mod exact_tensor_edit;
pub mod exact_surface_degree_reduction;
pub mod exact_surface_merge;
pub mod exact_curve_merge;
pub mod rational_gauge;
pub mod rational_factor;
mod rational_edit;
pub mod exact_knot_insertion;

pub mod exact_surface_knot_insertion;

pub mod rational_surface_factor;

pub mod exact_projective;

pub mod exact_derivative;

pub mod exact_surface_derivative;
