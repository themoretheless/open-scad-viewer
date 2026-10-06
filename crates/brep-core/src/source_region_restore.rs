//! Replay original region construction; serialized outcomes never admit material.
use crate::{
    source_contour_proposal::{self, SourceRegion},
    trimmed_face_recipe::{Boundary, Limits as RegionLimits},
};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use value_codec::{Deserialize, Serialize, Value, json};
pub struct Limits {
    /// Audit limits apply independently to every replay step.
    pub region: RegionLimits,
    pub search_cells: usize,
    pub point_checks: usize,
    pub mapping_cells: usize,
    pub driver_cells: usize,
    pub membership_cells: usize,
    pub controls: usize,
    pub winding_cells: usize,
    pub steps: usize,
}
fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_REGION_RESTORE", message)
}
fn decode<T: for<'de> Deserialize<'de>>(v: Value) -> Result<T> {
    T::from_value(v).map_err(|_| error("Invalid original region field"))
}
pub(crate) fn original_definition(
    context: &cad_predicates::ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    tolerance: f64,
) -> Value {
    let wires=wires.iter().map(|w|w.iter().map(|b|json!({"curve":b.curve.to_value(),"pcurve":b.pcurve.to_value(),"reversed":b.reversed})).collect::<Vec<_>>()).collect::<Vec<_>>();
    json!({"version":1,"kind":"original","context":context.to_value(),"surface":surface.to_value(),"wires":wires,"toleranceUV":tolerance})
}
pub(crate) fn cut_definition(
    context: &cad_predicates::ToleranceContext,
    kind: &str,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    contact: &Curve,
    address: [usize; 3],
    tolerance: f64,
    target_width: [f64; 2],
    axis: usize,
) -> Value {
    let mut value = original_definition(context, surface, wires, tolerance);
    value["kind"] = json!(kind);
    value["contact"] = contact.to_value();
    value["address"] = json!(address);
    value["targetWidth"] = json!(target_width);
    value["driverAxis"] = json!(axis);
    value
}
fn region_limits(l: &Limits) -> RegionLimits {
    RegionLimits {
        pairs: l.region.pairs,
        region_cells: l.region.region_cells,
        domain_cells: l.region.domain_cells,
        agreement_cells: l.region.agreement_cells,
    }
}
pub fn restore(value: Value, limits: &Limits) -> Result<SourceRegion> {
    if !(1..=64).contains(&limits.steps) {
        return Err(error("Bound region replay depth to 1..64"));
    }
    replay(value, limits, limits.steps)
}
fn replay(value: Value, l: &Limits, remaining: usize) -> Result<SourceRegion> {
    if remaining == 0 || value["version"].as_u64() != Some(1) {
        return Err(error("Region replay depth or version refused"));
    }
    let kind = value["kind"]
        .as_str()
        .ok_or_else(|| error("Missing region recipe"))?;
    if kind == "splitRoot" || kind == "splitParameter" {
        let parent = replay(value["parent"].clone(), l, remaining - 1)?;
        let [wire, edge]: [usize; 2] = decode(value["address"].clone())?;
        if kind == "splitParameter" {
            return parent.split_boundary_parameter(
                wire,
                edge,
                decode(value["parameter"].clone())?,
            );
        }
        let point = crate::source_contact_point::restore(value["point"].clone(), l.mapping_cells)?;
        let point = point
            .point
            .ok_or_else(|| error("Source split root is unqualified"))?;
        let role = match value["role"].as_str() {
            Some("boundary") => crate::source_boundary_fragment::Role::Boundary,
            Some("contact") => crate::source_boundary_fragment::Role::Contact,
            _ => return Err(error("Unknown root role")),
        };
        return parent.split_boundary(wire, edge, &point, role);
    }
    let surface: Surface = decode(value["surface"].clone())?;
    let arrays = value["wires"]
        .as_array()
        .filter(|v| !v.is_empty() && v.len() <= 16)
        .ok_or_else(|| error("Bound original wires"))?;
    let mut wires = Vec::new();
    for wire in arrays {
        let boundaries = wire
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 256)
            .ok_or_else(|| error("Bound original edges"))?;
        let mut out = Vec::new();
        for b in boundaries {
            out.push(Boundary {
                curve: decode(b["curve"].clone())?,
                pcurve: decode(b["pcurve"].clone())?,
                reversed: decode(b["reversed"].clone())?,
            });
        }
        wires.push(out);
    }
    let tolerance = decode(value["toleranceUV"].clone())?;
    let context: cad_predicates::ToleranceContext = decode(value["context"].clone())?;
    if kind == "original" {
        let report = source_contour_proposal::qualify_original_region(
            &context,
            &surface,
            &wires,
            tolerance,
            region_limits(l),
        )?;
        return report.region.ok_or_else(|| error(report.reason));
    }
    let contact: Curve = decode(value["contact"].clone())?;
    let [wire, start, end]: [usize; 3] = decode(value["address"].clone())?;
    let width = decode(value["targetWidth"].clone())?;
    let axis = decode(value["driverAxis"].clone())?;
    match kind {
        "linear" => {
            let report = source_contour_proposal::qualify_linear_region(
                &context,
                &surface,
                &wires,
                &contact,
                wire,
                start,
                end,
                tolerance,
                region_limits(l),
                l.search_cells,
                width,
                l.point_checks,
                l.mapping_cells,
                axis,
                l.driver_cells,
                l.membership_cells,
                l.controls,
            )?;
            report.region.ok_or_else(|| error(report.reason))
        }
        "curved" => {
            let report = source_contour_proposal::qualify_curved_region(
                &context,
                &surface,
                &wires,
                &contact,
                wire,
                start,
                end,
                tolerance,
                region_limits(l),
                l.search_cells,
                width,
                l.point_checks,
                l.mapping_cells,
                axis,
                l.driver_cells,
                l.membership_cells,
                l.winding_cells,
            )?;
            report.region.ok_or_else(|| error(report.reason))
        }
        _ => Err(error("Unknown original region recipe")),
    }
}

#[cfg(test)]
pub(crate) fn test_limits() -> Limits {
    Limits {
        region: RegionLimits {
            pairs: 100000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        },
        search_cells: 100000,
        point_checks: 100000,
        mapping_cells: 100000,
        driver_cells: 100000,
        membership_cells: 100000,
        controls: 100000,
        winding_cells: 100000,
        steps: 64,
    }
}
#[cfg(test)]
pub(crate) fn assert_replay(region: &SourceRegion) {
    let encoded = value_codec::to_string(&region.definition()).unwrap();
    let value: Value = value_codec::from_str(&encoded).unwrap();
    let replay = restore(value, &test_limits()).unwrap();
    assert_eq!(replay.definition(), region.definition());
    assert_eq!(replay.chart_winding(), region.chart_winding());
    assert_eq!(replay.whole_chart_material(), region.whole_chart_material());
    assert_eq!(replay.source_loop_indices(), region.source_loop_indices());
    let loops = |r: &SourceRegion| {
        r.loops()
            .iter()
            .map(|w| w.iter().map(|f| f.definition()).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    assert_eq!(loops(&replay), loops(region));
    let preview = crate::source_region_display::prepare(region, 4, 1e-8, 10000).unwrap();
    assert_eq!(
        preview,
        crate::source_region_display::prepare(&replay, 4, 1e-8, 10000).unwrap()
    );
    assert_eq!(
        preview.tiles.len() + preview.unresolved.len() + preview.outside,
        16
    );
    assert!(preview.domain_cells <= 10000);
    assert!(crate::source_region_display::prepare(region, 0, 1e-8, 10000).is_err());
    assert!(crate::source_region_display::prepare(region, 65, 1e-8, 10000).is_err());
    assert!(crate::source_region_display::prepare(region, 4, 1e-8, 0).is_err());
    if region.whole_chart_material() {
        assert_eq!(preview.tiles.len(), 16);
        assert!(preview.unresolved.is_empty());
    } else {
        assert!(!preview.unresolved.is_empty());
        let limited = crate::source_region_display::prepare(region, 4, 1e-8, 1).unwrap();
        assert!(limited.domain_cells <= 1);
        assert!(limited.tiles.is_empty());
        assert_eq!(limited.unresolved.len(), 16);
    }
}
