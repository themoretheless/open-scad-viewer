//! Fresh complete actual filled-map admission, independent of pairwise search.
use super::*;
use nurbs_core::affine_volume as atlas;
mod translated;
pub struct AffineFilledVolumeReport {
    pub certified: bool,
    pub packet_bound: bool,
    /// Fresh coefficient/UV binding plus complete actual lifts cancels exact endpoint translations.
    pub translated_trace_transport: bool,
    /// Exact invertible endpoint maps, same UV clocks and fresh full actual lifts.
    pub affine_trace_transport: bool,
    /// Distinct original carriers use a fresh normalized common material proof
    /// and a complete two-carrier filled map, rather than carrier equality.
    pub paired_carriers: bool,
    pub carrier_identity: Option<nurbs_core::progressive_sweep::NormalizedCarrierIdentityReport>,
    pub material_roles_certified: bool,
    pub agreement: Option<crate::boundary_agreement::ExactReport>,
    pub trim: Option<crate::face_domain::TrimAudit>,
    pub volume: Option<nurbs_core::progressive_sweep::IdealVolumeReport>,
    pub orientation: Option<atlas::OrientationReport>,
    /// Shared UV cells + UV domain cells + filled atlas/orientation cells.
    pub cells: usize,
    /// Cumulative affine ledger, INCLUDING candidate preparation work.
    pub work: u64,
    pub reason: &'static str,
}
fn rectangle(model: &Model, face: usize) -> bool {
    let f = &model.faces[face];
    let uses = &model.loops[f.outer].coedges;
    if !f.holes.is_empty() || uses.len() != 4 {
        return false;
    }
    let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut seen = [false; 4];
    for c in uses {
        let p = &c.pcurve;
        if c.trim().is_some()
            || p.degree != 1
            || p.control_points.len() != 2
            || p.periodic
            || p.weights[0] != p.weights[1]
            || p.knots[..=1].iter().any(|t| *t != p.knots[1])
            || p.knots[2..].iter().any(|t| *t != p.knots[2])
        {
            return false;
        }
        let a = &p.control_points[0];
        let b = &p.control_points[1];
        let Some(i) = (0..4).find(|&i| {
            (a.as_slice() == corners[i] && b.as_slice() == corners[(i + 1) % 4])
                || (b.as_slice() == corners[i] && a.as_slice() == corners[(i + 1) % 4])
        }) else {
            return false;
        };
        if seen[i] {
            return false;
        }
        seen[i] = true;
    }
    seen.into_iter().all(|v| v)
}
// A constant coordinate and a nonconstant clamped Bernstein coordinate
// cannot be equal: positive weights and independence of the Bernstein basis
// make a nonzero homogeneous coefficient polynomial nonzero. This only avoids
// an unnecessary equality trial; the complete TWO-carrier theorem still runs.
fn distinct_planar_coordinate(
    caps: [&nurbs_core::surface::Surface; 2],
    out: &mut AffineFilledVolumeReport,
    max: u64,
) -> Result<bool> {
    for (a, b) in [(caps[0], caps[1]), (caps[1], caps[0])] {
        if b.periodic_u
            || b.periodic_v
            || b.control_points.len() != b.degree_u + 1
            || b.control_points[0].len() != b.degree_v + 1
        {
            continue;
        }
        if [(&b.knots_u, b.degree_u), (&b.knots_v, b.degree_v)]
            .iter()
            .any(|(k, p)| {
                !k[..=*p].iter().all(|x| *x == k[*p])
                    || !k[*p + 1..].iter().all(|x| *x == k[*p + 1])
            })
        {
            continue;
        }
        for axis in 0..3 {
            let value = a.control_points[0][0][axis];
            let mut constant = true;
            let mut different = false;
            for p in a.control_points.iter().flatten() {
                if out.work == max {
                    out.reason = "actual-affine-boundary-work-limit";
                    return Ok(false);
                }
                out.work += 1;
                constant &= p[axis] == value;
            }
            if !constant {
                continue;
            }
            for p in b.control_points.iter().flatten() {
                if out.work == max {
                    out.reason = "actual-affine-boundary-work-limit";
                    return Ok(false);
                }
                out.work += 1;
                different |= p[axis] != value;
            }
            if different {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
/// Only the private freshly constructed coherent packet can call this route.
/// Whole normalized carrier equality is freshly proved on the actual original
/// surfaces; unresolved representation equivalence supplies no authority.
pub(super) fn qualify(
    model: &Model,
    c: &affine_candidate::Candidate,
    source_caps: [&LoftCap; 2],
    tolerance_uv: f64,
    projection: [[f64; 3]; 3],
    limits: crate::AffineStationLimits,
    spent: u64,
) -> Result<AffineFilledVolumeReport> {
    let mut out = AffineFilledVolumeReport {
        certified: false,
        packet_bound: false,
        translated_trace_transport: false,
        affine_trace_transport: false,
        paired_carriers: false,
        carrier_identity: None,
        material_roles_certified: false,
        agreement: None,
        trim: None,
        volume: None,
        orientation: None,
        cells: 0,
        work: spent,
        reason: "actual-affine-boundary-packet-unproved",
    };
    if c.nominal_surfaces[0] != source_caps[0].surface
        || c.nominal_surfaces[1] != source_caps[1].surface
    {
        out.reason = "actual-affine-source-packet-rebound";
        return Ok(out);
    }
    // The actual coefficient map may use a bounded corrected carrier. Nominal
    // sources above remain bound to the independent original/error owner.
    // Fresh normalized lifts below bind BOTH actual carriers to every wall.
    let actual_caps = [
        LoftCap {
            surface: c.source_surfaces[0].clone(),
            trims: source_caps[0].trims.clone(),
        },
        LoftCap {
            surface: c.source_surfaces[1].clone(),
            trims: source_caps[1].trims.clone(),
        },
    ];
    let source_caps = [&actual_caps[0], &actual_caps[1]];
    use nurbs_core::progressive_sweep::{
        NormalizedCarrierIdentityAttempt, NormalizedCarrierIdentityLimits,
        certify_normalized_carrier_identity,
    };
    let caps = [&source_caps[0].surface, &source_caps[1].surface];
    let visibly_distinct = distinct_planar_coordinate(caps, &mut out, limits.work)?;
    let same_carrier = if visibly_distinct {
        false
    } else {
        let identity = certify_normalized_carrier_identity(
            caps,
            NormalizedCarrierIdentityLimits {
                cells: limits.matrix_cells - out.cells,
                exact_work: limits.work - out.work,
            },
        )?;
        let same_carrier = match identity {
            NormalizedCarrierIdentityAttempt::Certified(p) => {
                out.cells += p.report().cells;
                out.work += p.report().exact_work;
                out.carrier_identity = Some(p.report().clone());
                p.matches(caps)
            }
            NormalizedCarrierIdentityAttempt::Refused(r) => {
                out.cells += r.cells;
                out.work += r.exact_work;
                out.carrier_identity = Some(*r);
                false
            }
        };
        same_carrier
    };
    if !same_carrier
        && !super::common_trim_region::certify(source_caps, &mut out.work, limits.work)?
    {
        out.reason = "actual-affine-common-material-domain-unproved";
        return Ok(out);
    }
    if model.shells.len() != 1
        || model.bodies.len() != 1
        || model.bodies[0].outer_shell != 0
        || !model.bodies[0].inner_shells.is_empty()
        || !model.shells[0].closed
    {
        return Ok(out);
    }
    let sides = c.sides.iter().flatten().collect::<Vec<_>>();
    let n = sides.len();
    if model.faces.len() != n + 2
        || model.shells[0].faces.len() != n + 2
        || model.faces[n].surface != c.caps[0].surface
        || model.faces[n + 1].surface != c.caps[1].surface
    {
        return Ok(out);
    }
    for (i, s) in sides.iter().enumerate() {
        if out.work == limits.work {
            out.reason = "actual-affine-boundary-work-limit";
            return Ok(out);
        }
        out.work += 1;
        if model.faces[i].surface != **s || !rectangle(model, i) {
            return Ok(out);
        }
    }
    model.validate()?;
    // Bind every cap occurrence to the actual affine source section, including
    // complete parameter clocks; fresh edge lifts below cannot be borrowed from
    // another model. No caller-supplied successful report is accepted.
    for (endpoint, expected) in [&c.start, &c.end].iter().enumerate() {
        let face = &model.faces[n + endpoint];
        let wires = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect::<Vec<_>>();
        if wires.len() != expected.len() {
            return Ok(out);
        }
        for (ring, &wire) in wires.iter().enumerate() {
            let uses = &model.loops[wire].coedges;
            if uses.len() != expected[ring].len() {
                return Ok(out);
            }
            for (coedge, profile) in uses.iter().zip(&expected[ring]) {
                if out.work == limits.work {
                    out.reason = "actual-affine-boundary-work-limit";
                    return Ok(out);
                }
                out.work += 1;
                if coedge.trim().is_some() {
                    return Ok(out);
                }
                let original = &model.edges[coedge.edge].curve;
                let actual = if coedge.reversed {
                    original.reverse()?
                } else {
                    original.clone()
                };
                if actual != *profile {
                    return Ok(out);
                }
            }
        }
    }
    let Some(translated_traces) = translated::prepare(model, c, source_caps, &mut out, limits.work)
    else {
        return Ok(out);
    };
    if !translated_traces {
        for source_cap in source_caps {
            if source_cap.trims.len() != c.original.len() {
                return Ok(out);
            }
            for (profiles, trims) in c.original.iter().zip(&source_cap.trims) {
                if profiles.len() != trims.len() {
                    return Ok(out);
                }
                for (world, uv) in profiles.iter().zip(trims) {
                    if out.work == limits.work {
                        out.reason = "actual-affine-boundary-work-limit";
                        return Ok(out);
                    }
                    let proof = nurbs_core::curve_surface_agreement::verify_exact(
                        world,
                        uv,
                        &source_cap.surface,
                        false,
                        limits.work - out.work,
                    )?;
                    let Some(proof) = proof else {
                        out.work = limits.work;
                        out.reason = "actual-affine-source-trace-unproved";
                        return Ok(out);
                    };
                    out.work += proof.work_used;
                    if proof.outcome != cad_predicates::BezierIdentity::Equal {
                        out.reason = "actual-affine-source-trace-unproved";
                        return Ok(out);
                    }
                }
            }
        }
    }
    if out.work == limits.work {
        out.reason = "actual-affine-boundary-work-limit";
        return Ok(out);
    }
    let agreement = crate::boundary_agreement::verify_exact(model, limits.work - out.work)?;
    out.work += agreement.work;
    let equal = agreement.all_equal && agreement.all_joins_exact;
    out.agreement = Some(agreement);
    if !equal {
        out.reason = "actual-affine-boundary-agreement-unproved";
        return Ok(out);
    }
    out.affine_trace_transport = translated_traces;
    out.translated_trace_transport = translated_traces
        && [
            c.pieces.first().unwrap().controls.first().unwrap(),
            c.pieces.last().unwrap().controls.last().unwrap(),
        ]
        .iter()
        .all(|p| p[..3] == [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
    let remaining = limits.matrix_cells - out.cells;
    if remaining < 2 || out.work == limits.work {
        out.reason = "actual-affine-boundary-work-limit";
        return Ok(out);
    }
    let cells = (remaining / 2).min(100000);
    let domain = (remaining - cells).min(1000000);
    let trim = crate::face_domain::audit_trim_regions(
        model,
        tolerance_uv,
        ((limits.work - out.work).min(100000)) as usize,
        cells,
        domain,
    )?;
    out.cells += trim.cells + trim.domain_cells;
    out.work += trim.pairs as u64;
    let valid = trim.all_valid;
    out.trim = Some(trim);
    if !valid {
        out.reason = "actual-affine-material-roles-unproved";
        return Ok(out);
    }
    for face in 0..n + 2 {
        let Some(trim) = out.trim.as_ref().unwrap().faces[face].as_ref() else {
            return Ok(out);
        };
        let rings = if face < n { 1 } else { c.start.len() };
        if trim.valid != Some(true)
            || trim.winding.len() != rings
            || trim.winding.first() != Some(&Some(1))
            || trim.winding.iter().skip(1).any(|w| *w != Some(-1))
        {
            out.reason = "actual-affine-material-roles-unproved";
            return Ok(out);
        }
    }
    out.packet_bound = true;
    // Fresh source identities bind BOTH normalized material traces to P.
    // Exact actual edges bind their endpoint images to A(0)P and A(1)P.
    // Equal carriers retain the original proof. Distinct carriers require the
    // fresh positive-affine UV correspondence above, so their normalized trim
    // clocks and regions coincide. Their Cartesian interpolation equals P on
    // every material boundary. The complete filled-map proof below supplies
    // global injectivity/rank; no pair is excluded merely by shared topology.
    let volume = if same_carrier {
        atlas::certify_rational(
            &source_caps[0].surface,
            c.origin,
            &c.pieces,
            false,
            projection,
            limits.matrix_cells - out.cells,
            limits.work - out.work,
        )?
    } else {
        atlas::certify_rational_caps(
            caps,
            c.origin,
            &c.pieces,
            projection,
            limits.matrix_cells - out.cells,
            limits.work - out.work,
        )?
    };
    out.cells += volume.cells;
    out.work += volume.exact_work;
    let proven = volume.embedding_certified
        && volume.regularity_certified
        && volume.parameter_topology_preserved;
    out.volume = Some(volume);
    if !proven {
        out.reason = "actual-affine-filled-volume-unproved";
        return Ok(out);
    }
    let orientation = if same_carrier {
        atlas::jacobian_orientation_rational(
            &source_caps[0].surface,
            c.origin,
            &c.pieces,
            [0.5; 3],
            limits.matrix_cells - out.cells,
        )?
    } else {
        atlas::jacobian_orientation_rational_caps(
            caps,
            c.origin,
            &c.pieces,
            [0.5; 3],
            limits.matrix_cells - out.cells,
        )?
    };
    out.cells += orientation.cells;
    let sign = orientation.sign;
    out.orientation = Some(orientation);
    let Some(sign) = sign else {
        out.reason = "actual-affine-orientation-unproved";
        return Ok(out);
    };
    let mut seen = vec![false; n + 2];
    for use_ in &model.shells[0].faces {
        if out.work == limits.work {
            out.reason = "actual-affine-boundary-work-limit";
            return Ok(out);
        }
        out.work += 1;
        if use_.face >= seen.len() || seen[use_.face] {
            return Ok(out);
        }
        seen[use_.face] = true;
        // Forward UV outer/hole traces transport the product orientation:
        // longitudinal normals are profile x traversal, lower cap is opposite.
        let reversed = if use_.face == n { sign > 0 } else { sign < 0 };
        if use_.reversed != reversed {
            out.reason = "actual-affine-material-orientation-unproved";
            return Ok(out);
        }
    }
    if !seen.into_iter().all(|v| v) {
        return Ok(out);
    }
    out.material_roles_certified = true;
    out.paired_carriers = !same_carrier;
    out.certified = true;
    out.reason = "actual-affine-filled-material-boundary-certified";
    Ok(out)
}

#[cfg(test)]
mod tests;
