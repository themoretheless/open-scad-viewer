//! Original affine-patch expressions through scalar isolation and finite UV admission.
use super::affine_plane::{enclosure, q, subtract};
use super::*;
use cad_predicates::{
    rational_bspline_affine_patch_edge_sign, rational_bspline_affine_patch_roots, AuthoredScalar,
    Limits, OriginalPlaneRootsDecision, OriginalPlaneRootsOutcome, PredicateContext, Sign,
    SourceArena, ToleranceContext,
};
use num_traits::Zero;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPatchUvProof {
    SeparableProjective,
    InjectiveBilinearKrawczyk,
    PositiveBilinearGlobalInverse,
    ExactBoundaryProjective,
    OriginalAffineCoordinateBounds,
    OriginalPolynomialCommonRoot,
    OriginalPolynomialCoordinateSign,
}
#[derive(Debug)]
pub struct OriginalPatchContact {
    pub uv_proof: OriginalPatchUvProof,
    pub parameter_box: Box3,
    pub position_bounds: [[f64; 2]; 3],
    /// Original one-sided spans and their exact scalar multiplicities.
    pub source_spans: Vec<usize>,
    pub multiplicities: Vec<usize>,
}
#[derive(Debug)]
pub struct OriginalPatchContacts {
    pub contacts: Vec<OriginalPatchContact>,
    pub unresolved_spans: Vec<usize>,
    pub ambiguous_parameters: Vec<[f64; 2]>,
    pub scalar_proofs: Vec<(usize, OriginalPlaneRootsDecision)>,
    pub exact_work: u64,
    pub spans: usize,
    pub complete: bool,
    pub target_weights_separable: bool,
    pub target_uv_injectivity: Option<super::patch_uv_injectivity::BilinearUvInjectivity>,
}
/// All original curve contacts with an exact untrimmed parallelogram support.
/// Positive separable target weights give independent projective UV clocks.
/// The plane is an expression of the original vertices, so derived normal and
/// offset need not be representable as binary64. Source continuity is required
/// at interior knots. Non-singleton boundary membership and coincident spans
/// stay unresolved.
/// This is neither a freeform tangency solver nor a trim/Body admission.
pub fn original_affine_patch_contacts(
    c: &Curve,
    s: &Surface,
    tolerance: f64,
    max_spans: usize,
    max_exact_work: u64,
) -> Result<OriginalPatchContacts> {
    c.validate()?;
    s.validate()?;
    check(
        !c.periodic
            && !s.periodic_u
            && !s.periodic_v
            && c.control_points[0].len() == 3
            && s.control_points[0][0].len() == 3,
        "Original patch contacts need nonperiodic 3D sources",
    )?;
    check(
        (1..=25).contains(&c.degree)
            && c.control_points.len() <= 256
            && (1..=100000).contains(&max_spans)
            && tolerance.is_finite()
            && tolerance > 0.,
        "Invalid original patch contact limits",
    )?;
    let [a, b] = c.domain();
    check(
        c.knots
            .iter()
            .filter(|&&t| a < t && t < b)
            .all(|t| c.knots.iter().filter(|v| *v == t).count() <= c.degree),
        "Original patch contacts require C0 source knots",
    )?;
    check(
        s.degree_u == 1
            && s.degree_v == 1
            && s.control_points.len() == 2
            && s.control_points[0].len() == 2
            && s.knots_u[0] == s.knots_u[1]
            && s.knots_u[2] == s.knots_u[3]
            && s.knots_v[0] == s.knots_v[1]
            && s.knots_v[2] == s.knots_v[3],
        "Target must be one clamped degree-(1,1) patch",
    )?;
    let separable =
        q(s.weights[0][0]) * q(s.weights[1][1]) == q(s.weights[0][1]) * q(s.weights[1][0]);
    let pole = |i: usize, j: usize| std::array::from_fn(|k| q(s.control_points[i][j][k]));
    let origin = pole(0, 0);
    let u = subtract(&pole(1, 0), &origin);
    let v = subtract(&pole(0, 1), &origin);
    check(
        subtract(&pole(1, 1), &pole(1, 0)) == v,
        "Target is not an exact parallelogram",
    )?;
    let (i, j, det) = [(0, 1), (0, 2), (1, 2)]
        .into_iter()
        .map(|(i, j)| (i, j, &u[i] * &v[j] - &u[j] * &v[i]))
        .find(|(_, _, d)| !d.is_zero())
        .ok_or_else(|| crate::input("Degenerate affine patch"))?;
    let bound = |x| {
        enclosure(x).ok_or_else(|| crate::numeric_err("No original affine coordinate enclosure"))
    };
    let determinant = bound(&det)?;
    let oi = bound(&origin[i])?;
    let oj = bound(&origin[j])?;
    let ui = bound(&u[i])?;
    let uj = bound(&u[j])?;
    let vi = bound(&v[i])?;
    let vj = bound(&v[j])?;
    let mut values: Vec<f64> = c
        .control_points
        .iter()
        .zip(&c.weights)
        .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
        .collect();
    let knot_start = values.len();
    values.extend(&c.knots);
    let patch_start = values.len();
    for (i, j) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        values.extend(&s.control_points[i][j]);
    }
    let arena = SourceArena::authored(
        "nurbs-core/original-affine-patch-contacts",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Invalid original source arena"))?;
    let controls: Vec<_> = (0..c.control_points.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
        .collect();
    let knots: Vec<_> = (0..c.knots.len())
        .map(|i| arena.leaf(knot_start + i).unwrap())
        .collect();
    let patch = std::array::from_fn(|i| {
        std::array::from_fn(|j| arena.leaf(patch_start + i * 3 + j).unwrap())
    });
    let tol = ToleranceContext::default_valid();
    let mut out = OriginalPatchContacts {
        contacts: vec![],
        unresolved_spans: vec![],
        ambiguous_parameters: vec![],
        scalar_proofs: vec![],
        exact_work: 0,
        spans: 0,
        complete: false,
        target_weights_separable: separable,
        target_uv_injectivity: None,
    };
    for span in c.degree..c.control_points.len() {
        let domain = [c.knots[span], c.knots[span + 1]];
        if domain[0] >= domain[1] {
            continue;
        }
        if out.spans == max_spans || out.exact_work == max_exact_work {
            out.unresolved_spans.push(span);
            continue;
        }
        let width = I::point(domain[1]).sub(I::point(domain[0]))?;
        let normalized = (I::point(tolerance).div(width)?.lo * 0.125).min(0.25);
        if normalized <= 0. || !normalized.is_finite() {
            out.unresolved_spans.push(span);
            continue;
        }
        let mut ctx = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: max_exact_work - out.exact_work,
                ..Limits::default()
            },
            None,
        );
        let decision = rational_bspline_affine_patch_roots(
            &mut ctx, &controls, &knots, c.degree, span, patch, normalized,
        )
        .map_err(|_| crate::input("Invalid original patch scalar query"))?;
        out.exact_work += decision.work_used;
        out.spans += 1;
        let roots = match &decision.outcome {
            OriginalPlaneRootsOutcome::Roots(r) => r.clone(),
            _ => {
                out.unresolved_spans.push(span);
                vec![]
            }
        };
        out.scalar_proofs.push((span, decision));
        let mut coordinate_bounds = None;
        for root in roots {
            let t = if root.interval == [0., 0.] {
                I::point(domain[0])
            } else if root.interval == [1., 1.] {
                I::point(domain[1])
            } else {
                I::point(domain[0])
                    .add(width.mul(I::new(root.interval[0], root.interval[1])?)?)?
                    .intersect(domain[0], domain[1])?
            };
            let p = crate::curve_distance::enclosure(c, span, t)?;
            let di = p[i].sub(oi)?;
            let dj = p[j].sub(oj)?;
            let alpha = di.mul(vj)?.sub(dj.mul(vi)?)?.div_signed(determinant)?;
            let beta = ui.mul(dj)?.sub(uj.mul(di)?)?.div_signed(determinant)?;
            if alpha.hi < 0. || alpha.lo > 1. || beta.hi < 0. || beta.lo > 1. {
                continue;
            }
            if alpha.lo <= 0. || alpha.hi >= 1. || beta.lo <= 0. || beta.hi >= 1. {
                if root.interval[0] == root.interval[1] {
                    if let Some(contact) = super::patch_exact_membership::contact(
                        c,
                        s,
                        span,
                        root.interval[0],
                        root.multiplicity,
                    )? {
                        if out.target_uv_injectivity.is_none() {
                            out.target_uv_injectivity =
                                Some(super::patch_uv_injectivity::certify(&s.weights)?);
                        }
                        merge_contact(&mut out.contacts, contact);
                    }
                } else {
                    if coordinate_bounds.is_none() {
                        coordinate_bounds =
                            Some(super::patch_coordinate_bounds::original(c, s, span)?);
                    }
                    let mut bounds = coordinate_bounds.as_ref().unwrap().clone();
                    let mut common_root_proven = false;
                    let mut coordinate_sign_proven = false;
                    let mut proven_outside = false;
                    if matches!(
                        bounds.contact(s, span, root.multiplicity, t, &p, alpha, beta)?,
                        super::patch_coordinate_bounds::Decision::Unresolved
                    ) {
                        for edge in 0..4 {
                            let coordinate = [alpha, beta][edge / 2];
                            let side = (edge % 2) as f64;
                            if coordinate.lo > side || coordinate.hi < side {
                                continue;
                            }
                            let before = ctx.work_used();
                            let proof = rational_bspline_affine_patch_edge_sign(
                                &mut ctx,
                                cad_predicates::RationalSpanRefs {
                                    controls: &controls,
                                    knots: &knots,
                                    degree: c.degree,
                                    span,
                                },
                                patch,
                                edge,
                                root.interval,
                            )
                            .map_err(|_| crate::input("Invalid original edge common-root query"))?;
                            out.exact_work += ctx.work_used() - before;
                            out.scalar_proofs.last_mut().unwrap().1.work_used = ctx.work_used();
                            match proof.sign {
                                Ok(Sign::Zero) => {
                                    bounds.constrain_edge(edge);
                                    common_root_proven = true;
                                }
                                Ok(sign) => {
                                    coordinate_sign_proven = true;
                                    if (edge % 2 == 0 && sign == Sign::Negative)
                                        || (edge % 2 == 1 && sign == Sign::Positive)
                                    {
                                        proven_outside = true;
                                        break;
                                    }
                                    bounds.constrain_inward(edge);
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    if proven_outside {
                        continue;
                    }
                    match bounds.contact(s, span, root.multiplicity, t, &p, alpha, beta)? {
                        super::patch_coordinate_bounds::Decision::Outside => {}
                        super::patch_coordinate_bounds::Decision::Unresolved => {
                            out.ambiguous_parameters.push([t.lo, t.hi])
                        }
                        super::patch_coordinate_bounds::Decision::Contact(mut contact) => {
                            if coordinate_sign_proven {
                                contact.uv_proof =
                                    OriginalPatchUvProof::OriginalPolynomialCoordinateSign;
                            } else if common_root_proven {
                                contact.uv_proof =
                                    OriginalPatchUvProof::OriginalPolynomialCommonRoot;
                            }
                            if out.target_uv_injectivity.is_none() {
                                out.target_uv_injectivity =
                                    Some(super::patch_uv_injectivity::certify(&s.weights)?);
                            }
                            merge_contact(&mut out.contacts, contact);
                        }
                    }
                }
                continue;
            }
            let natural =
                |x: I, k: &[f64]| I::point(k[1]).add(I::point(k[2]).sub(I::point(k[1]))?.mul(x)?);
            // Rank-one positive weights factor the tensor denominator. Invert
            // each original rational clock, not the Cartesian affine fraction.
            let clock = |x: I, w0: f64, w1: f64| -> Result<I> {
                if w0 == w1 {
                    return Ok(x);
                }
                let numerator = I::point(w0).mul(x)?;
                let denominator = I::point(w1).mul(I::point(1.).sub(x)?)?.add(numerator)?;
                numerator.div(denominator)?.intersect(0., 1.)
            };
            let mut global = false;
            let (uv, uv_proof) = if separable {
                let inverse = clock(alpha, s.weights[0][0], s.weights[1][0])
                    .and_then(|u| Ok([u, clock(beta, s.weights[0][0], s.weights[0][1])?]));
                match inverse {
                    Ok(uv) => (uv, OriginalPatchUvProof::SeparableProjective),
                    Err(e) if e.code == crate::NUMERIC_ERROR => {
                        global = true;
                        (
                            [I::new(0., 1.)?; 2],
                            OriginalPatchUvProof::PositiveBilinearGlobalInverse,
                        )
                    }
                    Err(e) => return Err(e),
                }
            } else {
                if out.target_uv_injectivity.is_none() {
                    out.target_uv_injectivity =
                        Some(super::patch_uv_injectivity::certify(&s.weights)?);
                }
                let inverse = match super::patch_uv::inverse(&s.weights, alpha, beta) {
                    Ok(v) => v,
                    Err(e) if e.code == crate::NUMERIC_ERROR => None,
                    Err(e) => return Err(e),
                };
                match inverse {
                    Some(b) => (
                        [I::new(b[0][0], b[0][1])?, I::new(b[1][0], b[1][1])?],
                        OriginalPatchUvProof::InjectiveBilinearKrawczyk,
                    ),
                    None => {
                        global = true;
                        (
                            [I::new(0., 1.)?; 2],
                            OriginalPatchUvProof::PositiveBilinearGlobalInverse,
                        )
                    }
                }
            };
            if global && out.target_uv_injectivity.is_none() {
                out.target_uv_injectivity = Some(super::patch_uv_injectivity::certify(&s.weights)?);
            }
            // Global bijectivity proves existence and uniqueness even when a
            // tight numeric inverse cannot be enclosed. Its whole-chart box
            // is honest evidence, not a tolerance-qualified authored vertex.
            let ub = if global {
                I::new(s.knots_u[1], s.knots_u[2])?
            } else {
                natural(uv[0], &s.knots_u)?.intersect(s.knots_u[1], s.knots_u[2])?
            };
            let vb = if global {
                I::new(s.knots_v[1], s.knots_v[2])?
            } else {
                natural(uv[1], &s.knots_v)?.intersect(s.knots_v[1], s.knots_v[2])?
            };
            let contact = OriginalPatchContact {
                uv_proof,
                parameter_box: [[t.lo, t.hi], [ub.lo, ub.hi], [vb.lo, vb.hi]],
                position_bounds: std::array::from_fn(|k| [p[k].lo, p[k].hi]),
                source_spans: vec![span],
                multiplicities: vec![root.multiplicity],
            };
            merge_contact(&mut out.contacts, contact);
        }
    }
    out.complete = out.unresolved_spans.is_empty() && out.ambiguous_parameters.is_empty();
    Ok(out)
}

// Only identical exact parameters alias. Overlapping interval roots stay separate.
fn merge_contact(contacts: &mut Vec<OriginalPatchContact>, contact: OriginalPatchContact) {
    let t = contact.parameter_box[0];
    if t[0] == t[1] {
        if let Some(previous) = contacts.iter_mut().find(|x| x.parameter_box[0] == t) {
            previous.source_spans.extend(contact.source_spans);
            previous.multiplicities.extend(contact.multiplicities);
            for k in 0..3 {
                previous.parameter_box[k][0] =
                    previous.parameter_box[k][0].min(contact.parameter_box[k][0]);
                previous.parameter_box[k][1] =
                    previous.parameter_box[k][1].max(contact.parameter_box[k][1]);
                previous.position_bounds[k][0] =
                    previous.position_bounds[k][0].min(contact.position_bounds[k][0]);
                previous.position_bounds[k][1] =
                    previous.position_bounds[k][1].max(contact.position_bounds[k][1]);
            }
            return;
        }
    }
    contacts.push(contact);
}
