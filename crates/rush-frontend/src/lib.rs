//! Bounded Rush frontend. No host evaluation or geometry execution.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
mod lower;
mod ordered_map;
mod parser;
mod value;
use value_codec::Value;

pub use math_core::{Error, Result};
pub(crate) const INVALID_INPUT: &str = "RUSH_FRONTEND_INVALID_INPUT";
pub(crate) fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}

/// Compile source into the portable RushGraph authoring graph and editor controls.
pub fn compile(source: &str) -> Result<Value> {
    let statements = parser::parse(source)?;
    lower::compile(statements)
}

#[cfg(test)]
mod miter_correction_tests {
    #[test]
    fn authored_cap_plane_mode_lowers_and_requires_boolean() {
        let source=include_str!("../../../examples/rush/miter-periodic-moving-axis-guide-affine-hollow-authored-caps.r");
        let compiled=super::compile_document(source).unwrap();
        let node=compiled["nodes"].as_array().unwrap().iter().find(|n|n["op"].as_str()==Some("brep_progressive_miter_sweep")).unwrap();
        assert_eq!(node["cap_correction_authored_frame"],value_codec::json!(true));
        assert!(super::compile_document(&source.replace("cap_correction_authored_frame: true","cap_correction_authored_frame: 2")).is_err());
    }

    #[test]
    fn moving_authored_guide_miter_lowers_explicit_chart_budget() {
        let source=include_str!("../../../examples/rush/miter-moving-frame-guide-affine-hollow-corrected.r");
        let compiled=super::compile_document(source).unwrap();
        let node=compiled["nodes"].as_array().unwrap().iter().find(|n|n["op"].as_str()==Some("brep_progressive_miter_sweep")).unwrap();
        assert_eq!(node["retained_wall_max_injectivity_cells"].as_f64(),Some(10000.));
        assert!(node["orientation_guide"].as_str().is_some());
        assert!(node["frame_axis"].is_object());
        assert!(node["axis_scale"].is_object());
    }
    #[test]
    fn miter_cap_correction_lowers_explicit_dimensional_budget() {
        let source=include_str!("../../../examples/rush/miter-hollow-corrected.r");
        let compiled=super::compile(source).unwrap();
        let node=compiled["nodes"].as_array().unwrap().iter().find(|n|n["op"].as_str()==Some("brep_miter_sweep")).unwrap();
        assert_eq!(node["cap_correction_tolerance"]["value"].as_f64(),Some(1e-9));
        assert_eq!(node["cap_correction_quantum"]["value"].as_f64(),Some(2_f64.powi(-40)));
        assert_eq!(node["cap_correction_max_work"].as_f64(),Some(1000000.));
    }
}

/// Rush declarative CAD compilation with IDs scoped to authored parts.
/// Independent declarations no longer renumber existing parts.
pub fn compile_declarative(source: &str) -> Result<Value> {
    lower::compile_with_ids(parser::parse(source)?, true)
}
/// Application dispatch preserving legacy RushGraph IDs.
pub fn compile_document(source: &str) -> Result<Value> {
    let first = source.trim_start();
    if first.strip_prefix("// @rush").is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.strip_prefix("/1").is_some_and(|tail| tail.is_empty() || tail.starts_with(char::is_whitespace))) {
        compile_declarative(source)
    } else { compile(source) }
}

#[cfg(test)]
mod circle_correction_tests {
    #[test]
    fn lowers_circle_correction_with_physical_units() {
        let source=include_str!("../../../examples/rush/progressive-miter-circle-corrected-hollow.r");
        let document=super::compile_document(source).unwrap();
        let node=document["nodes"].as_array().unwrap().iter().find(|n|n["op"].as_str()==Some("brep_progressive_miter_sweep")).unwrap();
        assert_eq!(node["circle_correction_tolerance"]["value"].as_f64(),Some(1e-10));
        assert_eq!(node["circle_correction_max_work"].as_f64(),Some(100000.));
        assert_eq!(node["circle_correction_tolerance"]["unit"].as_str(),Some("mm"));
    }
}

#[cfg(test)]
mod station_reconstruction_tests {
    #[test]
    fn lowers_station_reconstruction_as_a_brep_modifier_with_physical_budgets() {
        let document=super::compile_document(include_str!("../../../examples/rush/progressive-miter-reconstructed-stations.r")).unwrap();
        let node=document["nodes"].as_array().unwrap().iter().find(|n|n["op"].as_str()==Some("brep_smooth_miter_stations")).unwrap();
        assert!(node["input"].as_str().is_some());
        assert_eq!(node["wall_tolerance"]["value"].as_f64(),Some(0.5));
        assert_eq!(node["quantum"]["value"].as_f64(),Some(0.125));
        assert_eq!(node["max_deviation"]["unit"].as_str(),Some("mm"));
        assert_eq!(node["max_work"].as_f64(),Some(10000.));
    }
}

#[cfg(test)]
mod progressive_body_correction_tests {
    #[test]
    fn progressive_body_cap_correction_lowers_explicit_dimensional_budget(){
        for source in [include_str!("../../../examples/rush/arc-length-curved-fixed-normal-body-boundary.r"),
            include_str!("../../../examples/rush/arc-length-curved-fixed-normal-folded-body-boundary.r")]{
            let document=super::compile_document(source).unwrap();
            let node=document["nodes"].as_array().unwrap().iter().find(|n|n["op"].as_str()==Some("brep_progressive_sweep")).unwrap();
            assert_eq!(node["cap_correction_tolerance"]["value"].as_f64(),Some(1e-9));
            assert_eq!(node["cap_correction_quantum"]["value"].as_f64(),Some(2_f64.powi(-40)));
            assert_eq!(node["cap_correction_max_work"].as_f64(),Some(1000000.));
        }
    }
}

#[cfg(test)]
mod spatial_rmf_body_policy_tests {
    #[test]
    fn lowers_explicit_closed_spatial_rmf_body_policy() {
        let source=include_str!("../../../examples/rush/closed-spatial-rmf-affine-body-certified.r");
        let document=super::compile_document(source).unwrap();
        let node=document["nodes"].as_array().unwrap().iter().find(|n|n["op"].as_str()==Some("brep_progressive_sweep")).unwrap();
        assert_eq!(node["rmf_transport_steps"].as_f64(),Some(4096.));
        assert_eq!(node["error_max_cells"].as_f64(),Some(100000.));
        assert_eq!(node["error_max_products"].as_f64(),Some(1000000.));
    }
}
