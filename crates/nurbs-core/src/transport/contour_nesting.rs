use super::rotating_tools::xy_curve;
use super::*;
use crate::analysis::curve_measure::*;
use crate::chord_winding::FillRule;

pub(super) const OPS: &[&str] = &[
    "contour_placement", "contour_nesting", "contour_hierarchy", "contour_fill_orientations",
];

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let raw: Vec<crate::curve::Curve> = field(v, "curves")?;
    check((1..=32).contains(&raw.len()), "Expected one to 32 planar contours")?;
    let curves = raw.into_iter().map(xy_curve).collect::<Result<Vec<_>>>()?;
    let contours = curves.iter().collect::<Vec<_>>();
    let p = field(v, "parameter_tolerance")?;
    let d = field(v, "distance_tolerance_mm")?;
    let work = field(v, "max_cells")?;
    Ok(match op {
        "contour_placement" => {
            let n = field(v, "first_count")?;
            check(n > 0 && n < contours.len(), "Both contour sets require at least one curve")?;
            let (tested, region) = contours.split_at(n);
            let rule = if field(v, "even_odd")? { FillRule::EvenOdd } else { FillRule::NonZero };
            let r = classify_planar_contour_placement(tested, region, rule, p, d, work, work)?;
            let placements: Vec<_> = r.placements.into_iter().map(|row|row.map(|f|json!({"filled":f.filled,"winding":f.winding}))).collect();
            json!({"placements":placements,"distance":r.boundary_distance.bounds,"fillCells":r.fill_work_cells})
        }
        "contour_nesting" => {
            let r = analyze_planar_contour_nesting(&contours,p,d,work,work)?;
            let pairs: Vec<_> = r.pairs.into_iter().map(|row|json!({"contours":row.contours,"windings":row.windings,"distance":row.distance_bounds})).collect();
            json!({"pairs":pairs,"distanceCells":r.distance_work_cells,"windingCells":r.winding_work_cells})
        }
        "contour_hierarchy" => {
            let r = certify_planar_contour_hierarchy(&contours,p,d,work,work,work)?;
            json!({"parents":r.parents,"depths":r.depths,"simplicityCells":r.simplicity_cells})
        }
        "contour_fill_orientations" => {
            let r = certify_planar_contour_fill_orientations(&contours,p,d,work,work,work,work)?;
            json!({"reverse":r.reverse,"depths":r.hierarchy.depths,"orientationCells":r.orientation_work_cells})
        }
        _ => return Err(input("Unknown contour nesting operation")),
    })
}
