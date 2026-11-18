//! Root-bound restriction of actual contact definitions, before body ownership.
use crate::{Model, face_crossings, fillet_contact::Contact};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::Surface,
    surface_g2_pcurves,
};
pub struct Restriction {
    pub contact: Contact,
    /// Complete fresh root/membership reports for both actual cutter faces.
    pub coverage: [face_crossings::Report; 2],
    pub parameters: [f64; 2],
    /// One authored position per end, ready for shared vertex ownership.
    pub endpoints: [[f64; 3]; 2],
    pub root_position_error_upper: [f64; 2],
    pub original_agreement: curve_surface_agreement::Report,
    pub cells: usize,
    pub exact_work: u64,
}
fn invalid(s: &str) -> Error {
    Error::new("BREP_CONTACT_TRIM_INPUT", s)
}
fn unproven(s: &str) -> Error {
    Error::new("BREP_CONTACT_TRIM_UNPROVEN", s)
}
/// Cut between selected retained transverse roots, in increasing original curve
/// order. `cutters` contain model, face index and retained root index. Caller
/// reports never authorize trimming. No material-side, shared vertex identity,
/// radius, new G2 or closed-body certificate is implied by this operation.
pub fn between_faces(
    contact: &Contact,
    supports: [&Surface; 2],
    cutters: [(&Model, usize, usize); 2],
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Restriction> {
    if !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
    {
        return Err(invalid(
            "Need positive tolerances and bounded shared root/lift work",
        ));
    }
    contact.curve.validate()?;
    if contact.curve.periodic {
        return Err(unproven("Periodic root ownership requires a lifted branch"));
    }
    let mut cells = 0;
    let mut coverage = Vec::with_capacity(2);
    for &(model, face, _) in &cutters {
        if cells == max_cells {
            return Err(unproven("Shared contact root budget exhausted"));
        }
        let report = face_crossings::inspect_curve(
            &contact.curve,
            model,
            face,
            tolerance_uv,
            max_cells - cells,
        )?;
        cells += report.cells;
        if !report.complete {
            return Err(unproven(
                "Contact end root coverage or trim membership is unresolved",
            ));
        }
        coverage.push(report);
    }
    let roots = (0..2)
        .map(|k| {
            let index = *coverage[k]
                .eligible
                .get(cutters[k].2)
                .ok_or_else(|| invalid("Retained end root index is missing"))?;
            Ok(&coverage[k].roots[index].root)
        })
        .collect::<Result<Vec<_>>>()?;
    if roots[0].parameter_box[0][1] >= roots[1].parameter_box[0][0] {
        return Err(unproven(
            "End root bands must be strictly ordered and disjoint",
        ));
    }
    let domain = contact.curve.domain();
    let parameters = std::array::from_fn(|k| {
        crate::edge_partition::root_representative(roots[k].parameter_box[0], domain)
    });
    if !(domain[0] < parameters[0] && parameters[0] < parameters[1] && parameters[1] < domain[1]) {
        return Err(unproven(
            "Contact restriction needs two distinct interior root representatives",
        ));
    }
    let endpoints = parameters.map(|t| {
        contact
            .curve
            .evaluate(t)
            .map(|p| [p.point[0], p.point[1], p.point[2]])
    });
    let endpoints = [endpoints[0].clone()?, endpoints[1].clone()?];
    let mut errors = [0.; 2];
    for k in 0..2 {
        use nurbs_core::interval_eval::Interval as I;
        let mut squared = I::point(0.);
        for j in 0..3 {
            let b = roots[k].position_bounds[j];
            let d = I::point(endpoints[k][j]).sub(I::new(b[0], b[1])?)?;
            squared = squared.add(d.mul(d)?)?;
        }
        errors[k] = squared.hi.max(0.).sqrt().next_up();
        if errors[k] > tolerance_mm {
            return Err(unproven(
                "Authored contact end is not bounded to its original root",
            ));
        }
    }
    let mut curve = contact.curve.trim(parameters[0], parameters[1])?;
    let last = curve.control_points.len() - 1;
    curve.control_points[0] = endpoints[0].to_vec();
    curve.control_points[last] = endpoints[1].to_vec();
    let fractions = parameters.map(|t| (t - domain[0]) / (domain[1] - domain[0]));
    let restrict = |pc: &Curve| -> Result<Curve> {
        let [a, b] = pc.domain();
        pc.trim(a + (b - a) * fractions[0], a + (b - a) * fractions[1])
    };
    let support_pcurve = restrict(&contact.support_pcurve)?;
    let fillet_pcurve = restrict(&contact.fillet_pcurve)?;
    // C(u), independent of v, retains the ORIGINAL curve coefficients. It is
    // only a proof carrier, never a rounded cropped proxy or new support.
    let original = &contact.curve;
    let carrier = Surface {
        degree_u: original.degree,
        degree_v: 1,
        knots_u: original.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: original
            .control_points
            .iter()
            .map(|p| vec![p.clone(), p.clone()])
            .collect(),
        weights: original.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    if cells == max_cells {
        return Err(unproven("No original-curve agreement work remains"));
    }
    let map = Curve::from_polyline(vec![vec![parameters[0], 0.], vec![parameters[1], 0.]])?;
    let original_agreement = curve_surface_agreement::verify(
        &curve,
        &map,
        &carrier,
        false,
        tolerance_mm,
        max_cells - cells,
    )?;
    cells += original_agreement.cells;
    if original_agreement.status != Status::WithinTolerance {
        return Err(unproven(
            "Restricted contact disagrees with the original curve",
        ));
    }
    let positions = surface_g2_pcurves::verify_positions(
        &curve,
        [&support_pcurve, &fillet_pcurve],
        supports,
        [false, false],
        tolerance_mm,
        max_cells - cells,
        max_exact,
    )?;
    cells += positions.cells;
    if !positions.proven {
        return Err(unproven("Restricted contact lifts remain unproven"));
    }
    let exact_work = positions.exact_work;
    let mut reports = coverage.into_iter();
    Ok(Restriction {
        contact: Contact {
            curve,
            support_pcurve,
            fillet_pcurve,
            positions,
        },
        coverage: [reports.next().unwrap(), reports.next().unwrap()],
        parameters,
        endpoints,
        root_position_error_upper: errors,
        original_agreement,
        cells,
        exact_work,
    })
}
