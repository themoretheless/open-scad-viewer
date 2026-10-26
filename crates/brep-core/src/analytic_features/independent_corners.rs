use super::audited_feature_result::{
    certify_blend_result, is_axis_aligned_cuboid, model_bounds, refuse,
};
use super::*;
pub fn independent_cuboid_corner_fillets(
    model: &Model,
    radii: [Option<[f64; 2]>; 4],
) -> Result<AuditedFeatureResult> {
    const CODE: &str = "BREP_INDEPENDENT_CORNER_FILLET_REFUSED";
    audit_solid(model).map_err(|_| refuse(CODE, "Source must pass the solid audit"))?;
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            CODE,
            "Independent longitudinal corners currently require an axis-aligned cuboid",
        ));
    }
    let (min, max) = model_bounds(model);
    let selected = radii.map(|r| r.is_some());
    let bottom = radii.map(|r| r.map_or(0., |r| r[0]));
    let top = radii.map(|r| r.map_or(0., |r| r[1]));
    let result = crate::imprint_pipeline::independent_radius_cuboid_vertical_edges(
        model, min, max, selected, bottom, top,
    )
    .map_err(|e| refuse(CODE, &e.message))?;
    let scale = bottom.into_iter().chain(top).fold(0., f64::max);
    certify_blend_result(
        result,
        "independent-cuboid-longitudinal-fillets/1",
        vec![
            "independent_linear_radius_laws",
            "adjacent_setback_collision_check",
            "no_valence3_corner",
        ],
        scale,
        CODE,
    )
}

