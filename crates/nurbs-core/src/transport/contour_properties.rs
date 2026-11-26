use super::rotating_tools::xy_curve;
use super::*;
use crate::analysis::curve_measure::*;
use crate::chord_winding::FillRule;

pub(super) const OPS: &[&str] = &["orient_fill_contour", "contour_fill_area", "contour_fill_centroid", "contour_central_moments", "contour_principal_moments", "contour_principal_axes", "contour_nonzero_area", "contour_fill_perimeter", "contour_fill_topology", "contour_boundary_signed_distance"];

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let raw: Vec<crate::curve::Curve> = field(v, "curves")?;
    check((1..=32).contains(&raw.len()), "Expected one to 32 planar contours")?;
    let curves = raw.into_iter().map(xy_curve).collect::<Result<Vec<_>>>()?;
    let contours = curves.iter().collect::<Vec<_>>();
    let p = field(v, "parameter_tolerance")?;
    let d = field(v, "distance_tolerance_mm")?;
    let work = field(v, "max_cells")?;
    let rule = || -> Result<FillRule> {
        Ok(if field(v, "even_odd")? { FillRule::EvenOdd } else { FillRule::NonZero })
    };
    Ok(match op {
        "orient_fill_contour" => {
            let piece:usize=field(v,"piece")?;
            check(piece<contours.len(),"Selected contour is out of range")?;
            let r=orient_planar_contours_for_nonzero_fill(&contours,p,d,work,work,work,work,work as u64)?;
            let mut curve=r.contours.into_iter().nth(piece).ok_or_else(||input("Missing oriented contour"))?;
            for pt in &mut curve.control_points { pt.push(0.0); }
            json!({"curve":curve,"report":{"piece":piece,"reverse":r.orientation.reverse,"reversalWork":r.reversal_work}})
        }
        "contour_fill_area" => {
            let r=measure_planar_contour_fill_area(&contours,p,d,work,work,work,work)?;
            json!({"area":r.area_bounds,"signedAreas":r.signed_contour_area_bounds})
        }
        "contour_fill_centroid" => {
            let r=measure_planar_contour_fill_centroid(&contours,p,d,work,work,work,work)?;
            json!({"area":r.area_bounds,"centroid":r.centroid_bounds})
        }
        "contour_central_moments" => {
            let r=measure_planar_contour_fill_central_moments(&contours,p,d,work,work,work,work)?;
            json!({"centralMoments":r.central_moment_bounds,"centroid":r.centroid_bounds})
        }
        "contour_principal_moments" => {
            let r=measure_planar_contour_fill_principal_moments(&contours,p,d,work,work,work,work)?;
            json!({"moments":r.principal_moment_bounds,"gap":r.moment_gap_bounds})
        }
        "contour_principal_axes" => {
            let r=measure_planar_contour_fill_principal_axes(&contours,p,d,work,work,work,work)?;
            json!({"projectors":r.projector_bounds,"moments":r.moments.principal_moment_bounds})
        }
        "contour_nonzero_area" => {
            let r=measure_planar_contour_nonzero_fill_area(&contours,p,d,work,work,work,work)?;
            json!({"area":r.area_bounds,"windings":r.interior_windings,"multipliers":r.area_multipliers})
        }
        "contour_fill_perimeter" => {
            let r=measure_planar_contour_fill_perimeter(&contours,rule()?,p,d,work,work,work,work,work)?;
            json!({"perimeter":r.perimeter_bounds,"boundaryContours":r.boundary_contours,"converged":r.converged})
        }
        "contour_fill_topology" => {
            let r=analyze_planar_contour_fill_topology(&contours,rule()?,p,d,work,work,work,work)?;
            let components:Vec<_>=r.components.into_iter().map(|c|json!({"outer":c.outer,"holes":c.holes})).collect();
            json!({"components":components,"inactive":r.inactive_contours,"euler":r.euler_characteristic})
        }
        "contour_boundary_signed_distance" => {
            let r=measure_planar_contour_fill_boundary_signed_distance(&contours,field(v,"point")?,rule()?,p,d,work,work,work,work,work,work)?;
            json!({"signedDistance":r.signed_bounds,"filled":r.fill.filled,"witnessContour":r.witness_contour,"witnessParameter":r.witness_parameter})
        }
        _ => return Err(input("Unknown contour property")),
    })
}
