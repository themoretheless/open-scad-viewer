//! Whole-interval prescribed rolling-ball contact-center agreement.
//! This does not certify circular cross sections of a quintic G2 skin.
use crate::fillet_contact::Contact;
use nurbs_core::{
    Error, Result, interval_eval::Interval as I, offset::fillet_surfaces::RadiusLaw,
    surface::Surface, surface_g2_pcurves,
};
mod normal_trace;
use normal_trace::inspect as adaptive_bound;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    WithinTolerance,
    ExceedsTolerance,
    Unresolved,
}
pub struct Report {
    pub decision: Decision,
    pub reason: &'static str,
    /// Maximum contact-center discrepancy over the complete inspected intervals.
    pub maximum_center_gap_mm: Option<[f64; 2]>,
    /// Authored law endpoint envelope, not a measured skin curvature range.
    pub prescribed_radius_range_mm: Option<[f64; 2]>,
    /// Correction for independently isolated original contact parameters.
    pub parameter_drift_upper_mm: Option<f64>,
    /// Normalized contact parameter and a certified discrepancy lower bound.
    pub violation_witness: Option<[f64; 2]>,
    pub positions: Vec<surface_g2_pcurves::Positions>,
    pub cells: usize,
    pub exact_work: u64,
}
/// Compare A(t)+side_a*N_a(t)*r(t) with B(t)+side_b*N_b(t)*r(t).
/// Original support normals and fresh original/skin contact lifts are mandatory.
/// Constant/linear laws use polynomial B-spline linear precision plus a
/// conservative correction for the actual common rational weight spread.
/// Rounded Greville values are never substituted: original knot arithmetic is
/// enclosed outwards, and no original weight is snapped. Different trimmed
/// bases use whole-curve affine hull bounds including every interior pole.
/// Other laws, loose bounds or exhausted proof work remain unresolved.
pub fn inspect(
    supports: [&Surface; 2],
    skin: &Surface,
    contacts: [&Contact; 2],
    law: &RadiusLaw,
    sides: [f64; 2],
    tolerance_mm: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    contacts[0].curve.validate()?;
    let domain = contacts[0].curve.domain();
    inspect_contacts_in_domain(supports,skin,contacts,law,domain,sides,tolerance_mm,max_cells,max_exact)
}
/// Recheck independently trimmed contacts on their original radius-law domain.
/// Original coefficients, lifts and normals are inspected afresh. A trim does
/// not renormalize the original endpoint radii to its retained interval.
pub fn inspect_contacts_in_domain(
    supports:[&Surface;2],skin:&Surface,contacts:[&Contact;2],law:&RadiusLaw,
    original_law_domain:[f64;2],sides:[f64;2],tolerance_mm:f64,max_cells:usize,max_exact:u64,
)->Result<Report>{
    inspect_uses(
        supports,
        skin,
        contacts.map(|c| ContactUse {
            curve: &c.curve,
            support_pcurve: &c.support_pcurve,
            fillet_pcurve: &c.fillet_pcurve,
            reversed: [false; 2],
        }),
        law,
        original_law_domain,
        sides,
        tolerance_mm,
        max_cells,
        max_exact,
    )
}
#[derive(Clone, Copy)]
struct ContactUse<'a> {
    curve: &'a nurbs_core::curve::Curve,
    support_pcurve: &'a nurbs_core::curve::Curve,
    fillet_pcurve: &'a nurbs_core::curve::Curve,
    reversed: [bool; 2],
}
/// Fresh law inspection of actual saved B-rep coedges after trimming/grafting.
/// `law_domain` is the immutable original contact-curve parameter domain; it
/// must contain both current edge domains. Edge orientation and original face
/// charts come from actual topology, never rounded reconstructed pcurves.
/// `sides` are relative to each actual retained support chart's unit normal.
pub fn inspect_model(
    model: &crate::Model,
    skin_face: usize,
    edges: [usize; 2],
    law: &RadiusLaw,
    law_domain: [f64; 2],
    sides: [f64; 2],
    tolerance_mm: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    model.validate()?;
    if skin_face >= model.faces.len()
        || edges[0] == edges[1]
        || edges.iter().any(|&e| e >= model.edges.len())
    {
        return Err(Error::new(
            "BREP_FILLET_RADIUS_INPUT",
            "Need an existing skin and two distinct contact edges",
        ));
    }
    let mut uses = Vec::new();
    let mut faces = Vec::new();
    for edge in edges {
        let hits = model
            .faces
            .iter()
            .enumerate()
            .flat_map(|(f, face)| {
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(move |w| {
                        model.loops[w]
                            .coedges
                            .iter()
                            .filter(move |c| c.edge == edge)
                            .map(move |c| (f, c))
                    })
            })
            .collect::<Vec<_>>();
        if hits.len() != 2 || hits.iter().filter(|(f, _)| *f == skin_face).count() != 1 {
            return Err(Error::new(
                "BREP_FILLET_RADIUS_INPUT",
                "Each contact needs one skin use and one retained support use",
            ));
        }
        let (f, support) = hits.iter().copied().find(|(f, _)| *f != skin_face).unwrap();
        let (_, skin) = hits.iter().copied().find(|(f, _)| *f == skin_face).unwrap();
        faces.push(f);
        uses.push(ContactUse {
            curve: &model.edges[edge].curve,
            support_pcurve: &support.pcurve,
            fillet_pcurve: &skin.pcurve,
            reversed: [support.reversed, skin.reversed],
        });
    }
    inspect_uses(
        [
            &model.faces[faces[0]].surface,
            &model.faces[faces[1]].surface,
        ],
        &model.faces[skin_face].surface,
        [uses[0], uses[1]],
        law,
        law_domain,
        sides,
        tolerance_mm,
        max_cells,
        max_exact,
    )
}
fn inspect_uses(
    supports: [&Surface; 2],
    skin: &Surface,
    contacts: [ContactUse<'_>; 2],
    law: &RadiusLaw,
    law_domain: [f64; 2],
    sides: [f64; 2],
    tolerance_mm: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    for s in supports {
        s.validate()?;
    }
    skin.validate()?;
    for c in contacts {
        c.curve.validate()?;
        c.support_pcurve.validate()?;
        c.fillet_pcurve.validate()?;
    }
    if !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || max_cells > 100000
        || max_exact > 1000000
        || sides.iter().any(|x| !x.is_finite() || *x == 0.)
    {
        return Err(Error::new(
            "BREP_FILLET_RADIUS_INPUT",
            "Need finite positive tolerance and bounded center-law work",
        ));
    }
    let mut out = Report {
        decision: Decision::Unresolved,
        reason: "work-limit",
        maximum_center_gap_mm: None,
        prescribed_radius_range_mm: None,
        parameter_drift_upper_mm: None,
        violation_witness: None,
        positions: vec![],
        cells: 0,
        exact_work: 0,
    };
    let radii = match law {
        RadiusLaw::Constant(r) => [*r, *r],
        RadiusLaw::Linear { start, end } => [*start, *end],
        RadiusLaw::Spline(_) => {
            out.reason = "unsupported-spline-law";
            return Ok(out);
        }
    };
    if skin
        .control_points
        .len()
        .checked_mul(skin.control_points[0].len())
        .is_none_or(|n| n > 4096)
    {
        out.reason = "skin-control-budget";
        return Ok(out);
    }
    if radii.iter().any(|r| !r.is_finite() || *r <= 0.) {
        return Err(Error::new(
            "BREP_FILLET_RADIUS_INPUT",
            "Prescribed radii must be finite and positive",
        ));
    }
    out.prescribed_radius_range_mm = Some([radii[0].min(radii[1]), radii[0].max(radii[1])]);
    let a = contacts[0].curve;
    let b = contacts[1].curve;
    let n = a.control_points.len();
    let p = a.degree;
    let d = a.domain();
    if law_domain.iter().any(|x| !x.is_finite())
        || law_domain[0] >= law_domain[1]
        || d[0] < law_domain[0]
        || d[1] > law_domain[1]
    {
        return Err(Error::new(
            "BREP_FILLET_RADIUS_INPUT",
            "Current contacts must stay inside the original law domain",
        ));
    }
    for c in [a, b] {
        let domain = c.domain();
        let degree = c.degree;
        let count = c.control_points.len();
        if degree == 0
            || count > 4096
            || c.periodic
            || c.control_points[0].len() != 3
            || c.knots[..=degree].iter().any(|x| *x != domain[0])
            || c.knots[count..].iter().any(|x| *x != domain[1])
            || domain[0] < law_domain[0]
            || domain[1] > law_domain[1]
        {
            out.reason = "unsupported-contact-domain";
            return Ok(out);
        }
    }
    let shared_basis = a.degree == b.degree && a.knots == b.knots && a.weights == b.weights;
    let wmin = a.weights.iter().copied().fold(f64::INFINITY, f64::min);
    let wmax = a.weights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // For N_i>=0, sum N_i=1 and W=sum N_i*w_i:
    // |sum N_i*w_i*r_i/W - sum N_i*r_i| <= (wmax-wmin)*max|r_i|/wmin.
    // The difference of two unit support normals has norm at most two.
    let law_error = if wmin == wmax || radii[0] == radii[1] {
        0.
    } else {
        I::point(wmax)
            .sub(I::point(wmin))?
            .div(I::point(wmin))?
            .mul(I::point(radii[0].max(radii[1])))?
            .mul(I::point(2.))?
            .hi
    };
    let mut position_error = 0.;
    for side in 0..2 {
        if out.cells == max_cells {
            return Ok(out);
        }
        let c = contacts[side];
        let proof = surface_g2_pcurves::verify_positions(
            c.curve,
            [c.support_pcurve, c.fillet_pcurve],
            [supports[side], skin],
            c.reversed,
            tolerance_mm * 0.25,
            (max_cells - out.cells) / 2,
            max_exact - out.exact_work,
        )?;
        out.cells += proof.cells;
        out.exact_work += proof.exact_work;
        if !proof.proven {
            out.reason = "contact-lift-unproven";
            out.positions.push(proof);
            return Ok(out);
        }
        position_error = I::point(position_error)
            .add(I::point(proof.upper.unwrap()))?
            .hi;
        out.positions.push(proof);
    }
    let mut normals = Vec::new();
    for s in supports {
        if out.cells == max_cells {
            return Ok(out);
        }
        if s.control_points
            .len()
            .checked_mul(s.control_points[0].len())
            .is_none_or(|n| n > 4096)
        {
            out.reason = "support-control-budget";
            return Ok(out);
        }
        let domain = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let bound =
            nurbs_core::surface_shape_operator::rectangle_bounds(s, domain, max_cells - out.cells)?;
        out.cells += bound.cells;
        let Some(normal) = bound.shape.normal else {
            out.reason = "support-normal-unproven";
            return adaptive_bound(
                supports,
                contacts,
                radii,
                law_domain,
                sides,
                position_error,
                out,
                tolerance_mm,
                max_cells,
            );
        };
        normals.push(normal);
    }
    let direction = (0..3)
        .map(|k| {
            I::new(normals[0][k][0], normals[0][k][1])?
                .mul(I::point(sides[0].signum()))?
                .sub(I::new(normals[1][k][0], normals[1][k][1])?.mul(I::point(sides[1].signum()))?)
        })
        .collect::<Result<Vec<_>>>()?;
    if !shared_basis {
        let bounded = affine_hull_bound(
            [a, b],
            radii,
            law_domain,
            &direction,
            position_error,
            out,
            tolerance_mm,
            max_cells,
        )?;
        if bounded.decision != Decision::Unresolved {
            return Ok(bounded);
        }
        return adaptive_bound(
            supports,
            contacts,
            radii,
            law_domain,
            sides,
            position_error,
            bounded,
            tolerance_mm,
            max_cells,
        );
    }
    out.parameter_drift_upper_mm = Some(0.);
    let mut upper = 0_f64;
    let mut lower = 0_f64;
    for i in 0..n {
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let radius = if radii[0] == radii[1] {
            I::point(radii[0])
        } else {
            let mut g = I::point(0.);
            for &k in &a.knots[i + 1..i + p + 1] {
                g = g.add(I::point(k))?;
            }
            let t = g
                .div(I::point(p as f64))?
                .sub(I::point(law_domain[0]))?
                .div(I::point(law_domain[1]).sub(I::point(law_domain[0]))?)?;
            I::point(radii[0]).add(t.mul(I::point(radii[1]).sub(I::point(radii[0]))?)?)?
        };
        let mut squared = I::point(0.);
        for k in 0..3 {
            let residual = I::point(b.control_points[i][k])
                .sub(I::point(a.control_points[i][k]))?
                .sub(radius.mul(direction[k])?)?;
            squared = squared.add(residual.mul(residual)?)?;
        }
        upper = upper.max(squared.hi.max(0.).sqrt().next_up());
        // Only clamped endpoint coefficients are actual parameter witnesses.
        if i == 0 || i + 1 == n {
            lower = lower.max(squared.lo.max(0.).sqrt().next_down().max(0.));
        }
    }
    let lower = I::point(lower).sub(I::point(position_error))?.lo.max(0.);
    let upper = I::point(upper)
        .add(I::point(position_error))?
        .add(I::point(law_error))?
        .hi;
    out.maximum_center_gap_mm = Some([lower, upper]);
    out.decision = if upper <= tolerance_mm {
        Decision::WithinTolerance
    } else if lower > tolerance_mm {
        Decision::ExceedsTolerance
    } else {
        Decision::Unresolved
    };
    out.reason = match out.decision {
        Decision::WithinTolerance => "within-tolerance",
        Decision::ExceedsTolerance => "endpoint-exceeds-tolerance",
        Decision::Unresolved => "bound-overlaps-tolerance",
    };
    if out.decision == Decision::Unresolved {
        return adaptive_bound(
            supports,
            contacts,
            radii,
            law_domain,
            sides,
            position_error,
            out,
            tolerance_mm,
            max_cells,
        );
    }
    Ok(out)
}
fn norm_bounds(v: &[I]) -> Result<[f64; 2]> {
    let mut sum = I::point(0.);
    for &x in v {
        sum = sum.add(x.mul(x)?)?;
    }
    Ok([
        sum.lo.max(0.).sqrt().next_down().max(0.),
        sum.hi.max(0.).sqrt().next_up(),
    ])
}
/// Whole rational-curve deviation from its endpoint chord. Original knots and
/// weights are enclosed; the source curve is neither refitted nor normalized.
fn chord_error(c: &nurbs_core::curve::Curve, cells: &mut usize, max: usize) -> Result<Option<f64>> {
    let d = c.domain();
    let n = c.control_points.len();
    let p = c.degree;
    let start = &c.control_points[0];
    let end = &c.control_points[n - 1];
    let delta = (0..3)
        .map(|k| I::point(end[k]).sub(I::point(start[k])))
        .collect::<Result<Vec<_>>>()?;
    let mut error = 0_f64;
    for (i, point) in c.control_points.iter().enumerate() {
        if *cells == max {
            return Ok(None);
        }
        *cells += 1;
        let mut g = I::point(0.);
        for &k in &c.knots[i + 1..i + p + 1] {
            g = g.add(I::point(k))?;
        }
        let q = g
            .div(I::point(p as f64))?
            .sub(I::point(d[0]))?
            .div(I::point(d[1]).sub(I::point(d[0]))?)?;
        let residual = (0..3)
            .map(|k| I::point(point[k]).sub(I::point(start[k]).add(q.mul(delta[k])?)?))
            .collect::<Result<Vec<_>>>()?;
        error = error.max(norm_bounds(&residual)?[1]);
    }
    let wmin = c.weights.iter().copied().fold(f64::INFINITY, f64::min);
    let wmax = c.weights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let drift = if wmin == wmax {
        I::point(0.)
    } else {
        I::point(wmax)
            .sub(I::point(wmin))?
            .div(I::point(wmin))?
            .mul(I::point(norm_bounds(&delta)?[1]))?
    };
    Ok(Some(I::point(error).add(drift)?.hi))
}
/// Independent affine hulls provide a conservative fallback for different
/// trimmed knot bases. Curved contacts can remain unresolved; their entire
/// deviations are included, so endpoint agreement alone never grants admission.
fn affine_hull_bound(
    curves: [&nurbs_core::curve::Curve; 2],
    radii: [f64; 2],
    law_domain: [f64; 2],
    direction: &[I],
    position_error: f64,
    mut out: Report,
    tolerance: f64,
    max: usize,
) -> Result<Report> {
    let mut deviation = I::point(0.);
    // Pairing normalized current intervals preserves each original law map.
    // Their independently isolated root domains need not be bit-identical.
    let mut parameter_gap = 0_f64;
    for end in 0..2 {
        let gap = I::point(curves[0].domain()[end]).sub(I::point(curves[1].domain()[end]))?;
        parameter_gap = parameter_gap.max(gap.lo.abs().max(gap.hi.abs()));
    }
    let slope = I::point(radii[1])
        .sub(I::point(radii[0]))?
        .div(I::point(law_domain[1]).sub(I::point(law_domain[0]))?)?;
    let parameter_error = I::point(parameter_gap)
        .mul(I::point(slope.lo.abs().max(slope.hi.abs())))?
        .hi;
    out.parameter_drift_upper_mm = Some(parameter_error);
    deviation = deviation.add(I::point(parameter_error))?;
    for c in curves {
        let Some(error) = chord_error(c, &mut out.cells, max)? else {
            return Ok(out);
        };
        deviation = deviation.add(I::point(error))?;
    }
    let middle = direction
        .iter()
        .map(|x| x.lo * 0.5 + x.hi * 0.5)
        .collect::<Vec<_>>();
    let uncertainty = direction
        .iter()
        .zip(&middle)
        .map(|(&x, &m)| x.sub(I::point(m)))
        .collect::<Result<Vec<_>>>()?;
    deviation = deviation
        .add(I::point(norm_bounds(&uncertainty)?[1]).mul(I::point(radii[0].max(radii[1])))?)?;
    let mut upper = 0_f64;
    let mut lower = 0_f64;
    for end in 0..2 {
        if out.cells == max {
            return Ok(out);
        }
        out.cells += 1;
        let t = I::point(curves[0].domain()[end])
            .sub(I::point(law_domain[0]))?
            .div(I::point(law_domain[1]).sub(I::point(law_domain[0]))?)?;
        let radius = I::point(radii[0]).add(t.mul(I::point(radii[1]).sub(I::point(radii[0]))?)?)?;
        let point = |side: usize| {
            &curves[side].control_points[if end == 0 {
                0
            } else {
                curves[side].control_points.len() - 1
            }]
        };
        let residual = (0..3)
            .map(|k| {
                I::point(point(1)[k])
                    .sub(I::point(point(0)[k]))?
                    .sub(radius.mul(I::point(middle[k]))?)
            })
            .collect::<Result<Vec<_>>>()?;
        upper = upper.max(norm_bounds(&residual)?[1]);
        let witness = (0..3)
            .map(|k| {
                I::point(point(1)[k])
                    .sub(I::point(point(0)[k]))?
                    .sub(radius.mul(direction[k])?)
            })
            .collect::<Result<Vec<_>>>()?;
        lower = lower.max(norm_bounds(&witness)?[0]);
    }
    let upper = I::point(upper)
        .add(deviation)?
        .add(I::point(position_error))?
        .hi;
    let lower = I::point(lower)
        .sub(I::point(position_error))?
        .sub(I::point(parameter_error))?
        .lo
        .max(0.);
    out.maximum_center_gap_mm = Some([lower, upper]);
    out.decision = if upper <= tolerance {
        Decision::WithinTolerance
    } else if lower > tolerance {
        Decision::ExceedsTolerance
    } else {
        Decision::Unresolved
    };
    out.reason = match out.decision {
        Decision::WithinTolerance => "affine-hulls-within-tolerance",
        Decision::ExceedsTolerance => "endpoint-exceeds-tolerance",
        Decision::Unresolved => "affine-hull-bound-overlaps-tolerance",
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::curve::Curve;
    fn line(points: Vec<Vec<f64>>) -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![1.; 3],
            control_points: points,
            periodic: false,
        }
    }
    #[test]
    fn chord_bounds_keep_original_weights_and_include_every_interior_pole() {
        let mut curve = line(vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 0., 0.]]);
        let mut cells = 0;
        assert!(chord_error(&curve, &mut cells, 100).unwrap().unwrap() < 1e-12);
        curve.control_points[1][1] = 0.1;
        let saved = curve.clone();
        cells = 0;
        let error = chord_error(&curve, &mut cells, 100).unwrap().unwrap();
        assert!(error >= 0.05 && error < 0.100001);
        assert_eq!(curve, saved);
        curve.weights[1] = 2.;
        cells = 0;
        let error = chord_error(&curve, &mut cells, 100).unwrap().unwrap();
        let p = curve.evaluate(0.25).unwrap().point;
        assert!((p[0] - 0.25).hypot(p[1]) <= error);
        cells = 0;
        assert!(chord_error(&curve, &mut cells, 0).unwrap().is_none());
    }
    #[test]
    fn correct_endpoint_radii_cannot_hide_a_middle_contact_law_defect() {
        let a = line(vec![
            vec![0.5, 0., 0.],
            vec![1., 0.5, 0.],
            vec![1.5, 1., 0.],
        ]);
        let mut b = line(vec![
            vec![0., 0., 0.5],
            vec![0., 0.5, 1.],
            vec![0., 1., 1.5],
        ]);
        let report = || Report {
            decision: Decision::Unresolved,
            reason: "work-limit",
            maximum_center_gap_mm: None,
            prescribed_radius_range_mm: Some([0.5, 1.5]),
            parameter_drift_upper_mm: None,
            violation_witness: None,
            positions: vec![],
            cells: 0,
            exact_work: 0,
        };
        let direction = [I::point(-1.), I::point(0.), I::point(1.)];
        let valid = affine_hull_bound(
            [&a, &b],
            [0.5, 1.5],
            [0., 1.],
            &direction,
            0.,
            report(),
            1e-6,
            100,
        )
        .unwrap();
        assert_eq!(valid.decision, Decision::WithinTolerance);
        b.control_points[1][2] += 0.2;
        let defect = affine_hull_bound(
            [&a, &b],
            [0.5, 1.5],
            [0., 1.],
            &direction,
            0.,
            report(),
            1e-6,
            100,
        )
        .unwrap();
        assert_eq!(defect.decision, Decision::Unresolved);
        assert!(defect.maximum_center_gap_mm.unwrap()[1] >= 0.1);
        assert!(defect.maximum_center_gap_mm.unwrap()[0] <= 1e-6);
    }
}

/// Inspect the actual saved face coefficients, including every original knot
/// cell, against explicit world-coordinate center and radius fields. Previous
/// construction/contact-radius reports do not authorize this result. The face
/// must have one shell owner; closure/contact/G2 remain separate qualifications.
pub fn inspect_saved_skin(
    model:&crate::Model,
    skin_face:usize,
    center:[[f64;4];3],
    radius:[f64;4],
    tolerance_mm:f64,
    max_coefficients:usize,
)->Result<nurbs_core::skin_radius::Report>{
    model.validate()?;
    if skin_face>=model.faces.len() || tolerance_mm>model.tolerance_mm
        || model.shells.iter().flat_map(|s|&s.faces).filter(|f|f.face==skin_face).count()!=1 {
        return Err(Error::new("BREP_SAVED_SKIN_RADIUS_INPUT","Need one saved skin owner and unchanged model tolerance"));
    }
    nurbs_core::skin_radius::inspect_affine_field(&model.faces[skin_face].surface,center,radius,tolerance_mm,max_coefficients)
}
