//! Task-oriented views of the existing API. Original module paths remain supported.
//! Construction, validation and meshing keep their individual contracts and result types.

/// Existing modules for geometry.
pub mod geometry {
    pub use crate::curve;
    pub use crate::surface;
    pub use crate::foundation;
    pub use crate::bounds;
    pub use crate::conditioning;
    pub use crate::coordinate_frame;
    pub use crate::interval_eval;
    pub use crate::polynomial;
    pub use crate::compensated;
    pub use crate::dual;
    pub use crate::robust_solvers;
    pub use crate::interval_newton;
    pub use crate::sturm;
}

/// Existing modules for curves.
pub mod curves {
    pub use crate::curve_reconstruction;
    pub use crate::primitives;
    pub use crate::paths;
    pub use crate::closed_spline;
    pub use crate::natural_spline;
    pub use crate::hermite;
    pub use crate::engineering_profiles;
    pub use crate::helix;
    pub use crate::involute;
    pub use crate::logarithmic_spiral;
    pub use crate::lissajous;
    pub use crate::trochoid;
    pub use crate::circular_rolling;
    pub use crate::archimedean_spiral;
    pub use crate::catenary;
    pub use crate::toroidal_spiral;
    pub use crate::spherical_spiral;
    pub use crate::clothoid;
    pub use crate::biarc;
    pub use crate::curve_chain;
    pub use crate::rack;
    pub use crate::thread;
    pub use crate::formula;
}

/// Existing modules for surfaces.
pub mod surfaces {
    pub use crate::function_surface;
    pub use crate::boundary_fill;
    pub use crate::coons;
    pub use crate::gordon;
    pub use crate::grid_spline;
    pub use crate::framed_sweep;
    pub use crate::profile_sweep;
    pub use crate::progressive_sweep;
    pub use crate::catenoid;
    pub use crate::helicoid;
    pub use crate::screw_surface;
    pub use crate::pipe;
    pub use crate::ribbon;
    pub use crate::extrusion_patches;
    pub use crate::hermite_patch;
    pub use crate::natural_loft;
    pub use crate::guided_loft;
    pub use crate::loft_alignment;
    pub use crate::patches;
    pub use crate::scaled_sweep;
    pub use crate::triangular_patch;
    pub use crate::twist_sweep;
    pub use crate::two_guide_sweep;
    pub use crate::progressive_miter;
    pub use crate::helical_sweep;
    pub use crate::sections;
    pub use crate::section_projection;
    pub use crate::section_phase;
}

/// Existing modules for editing.
pub mod editing {
    pub use crate::affine;
    pub use crate::edit;
    pub use crate::surface_edit;
    pub use crate::weight_edit;
    pub use crate::canonical;
    pub use crate::direct_edit;
    pub use crate::morph;
    pub use crate::fairing;
    pub use crate::curve_extension;
    pub use crate::sweep_section_correction;
    pub use crate::circle_sweep_repair;
    pub use crate::miter_section_correction;
    pub use crate::sweep_section_projection;
    pub use crate::section_circle_repair;
}

/// Existing modules for joining.
pub mod joining {
    pub use crate::curve_join;
    pub use crate::continuity;
    pub use crate::loft_continuity;
    pub use crate::surface_join;
    pub use crate::surface_g1;
    pub use crate::surface_g2;
    pub use crate::circle_transition;
    pub use crate::ellipse_transition;
    pub use crate::circle_rectangle_transition;
    pub use crate::sweep_seam_set;
}

/// Existing modules for offsets.
pub mod offsets {
    pub use crate::curve_offset;
    pub use crate::curve_offset_join;
    pub use crate::curve_offset_wire;
    pub use crate::surface_offset;
    pub use crate::trimmed_offset_contact;
    pub use crate::offset_source_boundary;
    pub use crate::offset_contact_tangent;
    pub use crate::offset_envelope;
    pub use crate::offset_envelope_fit;
    pub use crate::offset_contact_pcurve;
    pub use crate::offset_contact_trims;
    pub use crate::offset_face_loops;
    pub use crate::offset_patch_boundary;
    pub use crate::offset_contact_predictor;
    pub use crate::offset_contact_path;
    pub use crate::offset_path_trims;
    pub use crate::offset_path_pcurves;
    pub use crate::moving_radius;
    pub use crate::moving_envelope;
}

/// Existing modules for queries.
pub mod queries {
    pub use crate::frechet;
    pub use crate::curve_differential;
    pub use crate::surface_differential;
    pub use crate::curve_measure;
    pub use crate::curve_extrema;
    pub use crate::curve_analysis;
    pub use crate::curve_distance;
    pub use crate::curve_deviation;
    pub use crate::curve_plane;
    pub use crate::curve_surface_distance;
    pub use crate::intersection;
    pub use crate::ray_surface;
    pub use crate::ss_intersection;
    pub use crate::surface_measure;
    pub use crate::surface_shape_operator;
    pub use crate::surface_contact;
    pub use crate::surface_contact_search;
    pub use crate::surface_distance;
    pub use crate::radial_bounds;
    pub use crate::geodesic;
    pub use crate::unrolling;
    pub use crate::uv_curve_crossings;
    pub use crate::planar_area;
    pub use crate::curve_axis_driver;
    pub use crate::surface_flux;
    pub use crate::convex_distance;
    pub use crate::obb_tree;
    pub use crate::normal_cone;
}

/// Existing modules for parameterization.
pub mod parameterization {
    pub use crate::curve_pullback;
    pub use crate::periodic_seam;
    pub use crate::trim_point;
    pub use crate::trim_domain;
    pub use crate::chord_arrangement;
    pub use crate::chord_embedding;
    pub use crate::chord_faces;
    pub use crate::chord_intersection;
    pub use crate::chord_winding;
    pub use crate::chord_witness;
    pub use crate::chord_fill_selection;
    pub use crate::surface_parameter_bounds;
    pub use crate::trimmed_surface_distance;
    pub use crate::loft_reparameterization;
    pub use crate::boundary_partition;
    pub use crate::bezier_extraction;
}

/// Existing modules for validation.
pub mod validation {
    pub use crate::curve_self_intersection;
    pub use crate::surface_self_intersection;
    pub use crate::curve_decomposition_certificate;
    pub use crate::curve_surface_agreement;
    pub use crate::curve_regularity;
    pub use crate::sweep_pair_audit;
    pub use crate::surface_injectivity;
    pub use crate::surface_regularity;
    pub use crate::surface_monotonicity;
    pub use crate::surface_linear_monotonicity;
    pub use crate::sweep_wall_audit;
    pub use crate::sweep_contour_audit;
    pub use crate::sweep_cap_boundary;
    pub use crate::sweep_cap_wall;
    pub use crate::sweep_seam_audit;
    pub use crate::trim_region_audit;
    pub use crate::trim_simplicity;
    pub use crate::retained_wall_coefficients;
    pub use crate::retained_wall_domain_certificate;
    pub use crate::surface_quotient_injectivity;
    pub use crate::normal_alignment;
    pub use crate::curve_surface_plane;
    pub use crate::curve_surface_affine;
    pub use crate::retained_wall_domain;
    pub use crate::curve_partition_agreement;
    pub use crate::contact_normal_agreement;
    pub use crate::curve_surface_lift;
    pub use crate::curve_point_identity;
    pub use crate::surface_projection_jacobian;
    pub use crate::curve_quadratic_separator;
    pub use crate::surface_projected_jordan;
    pub use crate::surface_self_chord;
    pub use crate::surface_self_chord_coverage;
}

/// Existing modules for meshing.
pub mod meshing {
    pub use crate::curve_tessellation;
    pub use crate::surface_tessellation;
    pub use crate::shared_tessellation;
}
