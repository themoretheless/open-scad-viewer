//! Curve constructors: primitives, profiles, spirals, interpolation splines.
pub use crate::editing::curve_extension;
pub mod airfoil;
pub mod archimedean_spiral;
pub mod biarc;
pub mod catenary;
pub mod circular_rolling;
pub mod clothoid;
pub use crate::foundation::closed_spline;
pub mod curve_chain;
pub mod curve_reconstruction;
pub mod engineering_profiles;
pub mod formula;
pub mod helix;
pub use crate::foundation::hermite;
pub mod involute;
pub mod lissajous;
pub mod logarithmic_spiral;
pub use crate::foundation::natural_spline;
pub mod paths;
pub mod primitives;
pub mod rack;
pub mod spherical_spiral;
pub mod thread;
pub mod toroidal_spiral;
pub mod trochoid;

pub mod radial_cam;

pub mod worm;
pub mod bounded_corner;
