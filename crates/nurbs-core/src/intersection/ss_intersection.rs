//! Certified bounded general NURBS surface/surface intersection.
//!
//! Isolates complete 4D parameter solution strata with outward-rounded
//! Bernstein/interval hull exclusion, Krawczyk uniqueness on terminals,
//! subdivision, and predictor-corrector continuation. Reports carry
//! ToleranceContext evidence, CoedgeTrim maps, BranchGraph components, and
//! revoke topology authority whenever unresolved/resource/conditioning bands
//! remain. Does not use the graph-patch iso fixture.
use crate::foundation::guards::{Budget, require_finite_point};
use crate::intersection::{
    ContactClass, MAX_BOXES, MAX_SPANS, UnresolvedReason, admit_surface, coedge_trim, context,
    cross3, distance, dot3, enclosure_of, homogeneous_grid, hull_diagonal, next_down, next_up,
    norm3, point3, split_grid_u, split_grid_v, surface_spans,
};
use crate::{Result, check, surface::Surface};
use cad_predicates::ToleranceContext;
#[path = "ss_intersection/planar.rs"]
mod planar;
#[path = "ss_intersection/samples.rs"]
mod samples;
pub use planar::{ExactBranch, ExactEndpointLocation, OverlapRegion};
use planar::{PlanarIntersection, plane_plane_line, surface_plane_iso_components};
#[path = "ss_intersection/unresolved.rs"]
mod unresolved;
pub use samples::ContinuationSample;
pub use unresolved::{UnresolvedClassification, UnresolvedSurfaceIntersection};
#[path = "ss_intersection/continuation.rs"]
mod continuation;
use continuation::continue_branch;
pub use continuation::{ContinuedBranch, EndpointLocation};
#[path = "ss_intersection/branches.rs"]
mod branches;
pub use branches::Summary as SurfaceIntersectionSummary;
#[path = "ss_intersection/junctions.rs"]
mod junctions;
#[path = "ss_intersection/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{intersect_surface_surface, ss_resource_probe, verify_ss_coverage};
#[path = "ss_intersection/report.rs"]
mod report;
use report::mark_junctions;
pub use report::{SurfaceSurfaceComponent, SurfaceSurfaceIntersection, TangencyBranch};

const TRANSVERSE_SINE: f64 = 1e-8;
const MAX_CONTINUATION: usize = 512;
const MAX_BRANCHES: usize = 256;

fn grids_excluded_ss(a: &[Vec<[f64; 4]>], b: &[Vec<[f64; 4]>]) -> bool {
    let flat_a: Vec<[f64; 4]> = a.iter().flatten().copied().collect();
    let flat_b: Vec<[f64; 4]> = b.iter().flatten().copied().collect();
    let ra = {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &flat_a {
            for axis in 0..3 {
                let x = p[axis] / p[3];
                lo[axis] = lo[axis].min(next_down(x));
                hi[axis] = hi[axis].max(next_up(x));
            }
        }
        [lo, hi]
    };
    let rb = {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &flat_b {
            for axis in 0..3 {
                let x = p[axis] / p[3];
                lo[axis] = lo[axis].min(next_down(x));
                hi[axis] = hi[axis].max(next_up(x));
            }
        }
        [lo, hi]
    };
    (0..3).any(|axis| ra[1][axis] < rb[0][axis] || rb[1][axis] < ra[0][axis])
}

fn proportional_grids(a: &[Vec<[f64; 4]>], b: &[Vec<[f64; 4]>], tol: f64) -> bool {
    if a.len() != b.len() || a.is_empty() || a[0].len() != b[0].len() {
        return false;
    }
    let mut scale = None;
    for (ra, rb) in a.iter().zip(b) {
        for (pa, pb) in ra.iter().zip(rb) {
            for axis in 0..4 {
                if pa[axis].abs() <= tol && pb[axis].abs() <= tol {
                    continue;
                }
                if pa[axis].abs() <= tol || pb[axis].abs() <= tol {
                    return false;
                }
                let ratio = pa[axis] / pb[axis];
                match scale {
                    None => scale = Some(ratio),
                    Some(s) if (ratio - s).abs() > tol.max(s.abs() * 1e-9) => return false,
                    _ => {}
                }
            }
        }
    }
    scale.is_some()
}

type PlaneCarrier = ([f64; 3], f64, [f64; 3], [f64; 3]);
type SeedRefinement = ([f64; 2], [f64; 2], [f64; 3], ContactClass);
type SurfaceCellPending = (
    [f64; 4],
    [f64; 4],
    Option<Vec<Vec<[f64; 4]>>>,
    Option<Vec<Vec<[f64; 4]>>>,
    usize,
);

fn affine_plane(surface: &Surface) -> Result<Option<PlaneCarrier>> {
    // Any positive-weight coplanar chart is an affine plane carrier, not only deg 1×1.
    let o = point3(&surface.control_points[0][0])?;
    let mut u_dir = None;
    let mut v_dir = None;
    for row in &surface.control_points {
        for corner in row {
            let p = point3(corner)?;
            let d = [p[0] - o[0], p[1] - o[1], p[2] - o[2]];
            if norm3(d) <= 1e-15 {
                continue;
            }
            if u_dir.is_none() {
                u_dir = Some(d);
                continue;
            }
            let u = u_dir.unwrap();
            let c = cross3(u, d);
            if norm3(c) > 1e-12 {
                v_dir = Some(d);
                break;
            }
        }
        if v_dir.is_some() {
            break;
        }
    }
    let (Some(u_dir), Some(v_dir)) = (u_dir, v_dir) else {
        return Ok(None);
    };
    let n = cross3(u_dir, v_dir);
    let nn = norm3(n);
    if !nn.is_finite() || nn <= 0. {
        return Ok(None);
    }
    let normal = n.map(|x| x / nn);
    let offset = dot3(normal, o);
    for row in &surface.control_points {
        for corner in row {
            let p = point3(corner)?;
            if (dot3(normal, p) - offset).abs() > 1e-9 {
                return Ok(None);
            }
        }
    }
    Ok(Some((normal, offset, u_dir, v_dir)))
}

fn invert_uv(
    origin: [f64; 3],
    u_dir: [f64; 3],
    v_dir: [f64; 3],
    domain: [[f64; 2]; 2],
    point: [f64; 3],
) -> Result<[f64; 2]> {
    let d = [
        point[0] - origin[0],
        point[1] - origin[1],
        point[2] - origin[2],
    ];
    let guu = dot3(u_dir, u_dir);
    let guv = dot3(u_dir, v_dir);
    let gvv = dot3(v_dir, v_dir);
    let det = guu * gvv - guv * guv;
    check(det.abs() > 0., "Degenerate planar frame")?;
    let ru = dot3(d, u_dir);
    let rv = dot3(d, v_dir);
    let su = (gvv * ru - guv * rv) / det;
    let sv = (-guv * ru + guu * rv) / det;
    Ok([
        domain[0][0] + su * (domain[0][1] - domain[0][0]),
        domain[1][0] + sv * (domain[1][1] - domain[1][0]),
    ])
}

fn surface_domain(surface: &Surface) -> [[f64; 2]; 2] {
    [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
        ],
    ]
}

fn ss_contact(
    first: &Surface,
    second: &Surface,
    uv: [f64; 2],
    st: [f64; 2],
    floor: f64,
) -> Result<ContactClass> {
    let ja = first.evaluate(uv[0], uv[1])?;
    let jb = second.evaluate(st[0], st[1])?;
    let Some((dua, dva)) = ja.first_derivatives() else {
        return Ok(ContactClass::PoleOrSingular);
    };
    let Some((dub, dvb)) = jb.first_derivatives() else {
        return Ok(ContactClass::PoleOrSingular);
    };
    let na = cross3(dua, dva);
    let nb = cross3(dub, dvb);
    let la = norm3(na);
    let lb = norm3(nb);
    if !la.is_finite() || !lb.is_finite() || la <= floor || lb <= floor {
        return Ok(ContactClass::PoleOrSingular);
    }
    let na = na.map(|x| x / la);
    let nb = nb.map(|x| x / lb);
    let sine = norm3(cross3(na, nb));
    if sine > TRANSVERSE_SINE {
        Ok(ContactClass::Transverse)
    } else if sine <= floor {
        Ok(ContactClass::EvenTangency)
    } else {
        Ok(ContactClass::OddTangency)
    }
}

fn refine_seed(
    first: &Surface,
    second: &Surface,
    uv: [f64; 2],
    st: [f64; 2],
    uv_box: [f64; 4],
    st_box: [f64; 4],
    floor: f64,
) -> Result<Option<SeedRefinement>> {
    let (mut u, mut v, mut s, mut t) = (uv[0], uv[1], st[0], st[1]);
    // Unified guard backing the fixed 16-step Newton polish (item 1065).
    let mut guard = Budget::with_iterations(16)?.guard("ssi_seed_newton");
    for _ in 0..16 {
        guard.tick()?;
        let ja = first.evaluate(u, v)?;
        let jb = second.evaluate(s, t)?;
        let Some((dua, dva)) = ja.first_derivatives() else {
            return Ok(None);
        };
        let Some((dub, dvb)) = jb.first_derivatives() else {
            return Ok(None);
        };
        let r = [
            ja.point[0] - jb.point[0],
            ja.point[1] - jb.point[1],
            ja.point[2] - jb.point[2],
        ];
        // Underdetermined 3×4 system; fix the free parameter along n1×n2 later.
        // Least-squares on the three spatial residuals w.r.t. (du,dv,ds) with dt from
        // projecting onto the intersection tangent after a provisional Newton step.
        let na = cross3(dua, dva);
        let nb = cross3(dub, dvb);
        let dir = cross3(na, nb);
        let dn = norm3(dir);
        let cols = [dua, dva, dub.map(|x| -x), dvb.map(|x| -x)];
        // Prefer the three columns with largest column norms excluding one free.
        let free = if dn > TRANSVERSE_SINE * norm3(na) * norm3(nb) {
            // Keep all four via QR-ish normal equations on first three + constraint.
            3usize
        } else {
            3
        };
        let use_cols = [0usize, 1, 2];
        let _ = free;
        let mut ata = [[0.; 3]; 3];
        let mut atb = [0.; 3];
        for i in 0..3 {
            for j in 0..3 {
                ata[i][j] = dot3(cols[use_cols[i]], cols[use_cols[j]]);
            }
            atb[i] = dot3(cols[use_cols[i]], r);
        }
        let det = ata[0][0] * (ata[1][1] * ata[2][2] - ata[1][2] * ata[2][1])
            - ata[0][1] * (ata[1][0] * ata[2][2] - ata[1][2] * ata[2][0])
            + ata[0][2] * (ata[1][0] * ata[2][1] - ata[1][1] * ata[2][0]);
        if !det.is_finite() || det.abs() <= 64. * f64::EPSILON {
            return Ok(None);
        }
        let mut delta = [0.; 3];
        for col in 0..3 {
            let mut m = ata;
            for row in 0..3 {
                m[row][col] = atb[row];
            }
            let d = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
            delta[col] = d / det;
        }
        u -= delta[0];
        v -= delta[1];
        s -= delta[2];
        // Adjust t by projecting residual onto remaining column.
        let ja2 = first.evaluate(u, v)?;
        let jb2 = second.evaluate(s, t)?;
        let r2 = [
            ja2.point[0] - jb2.point[0],
            ja2.point[1] - jb2.point[1],
            ja2.point[2] - jb2.point[2],
        ];
        let Some((_, _)) = ja2.first_derivatives() else {
            return Ok(None);
        };
        let Some((_, dvb2)) = jb2.first_derivatives() else {
            return Ok(None);
        };
        let denom = dot3(dvb2, dvb2);
        if denom > 0. {
            t += dot3(r2, dvb2) / denom;
        }
        if !(uv_box[0] <= u
            && u <= uv_box[1]
            && uv_box[2] <= v
            && v <= uv_box[3]
            && st_box[0] <= s
            && s <= st_box[1]
            && st_box[2] <= t
            && t <= st_box[3])
        {
            return Ok(None);
        }
        if delta.iter().copied().fold(0_f64, f64::max) <= floor {
            break;
        }
    }
    let ja = first.evaluate(u, v)?;
    let jb = second.evaluate(s, t)?;
    let pa = point3(&ja.point)?;
    let pb = point3(&jb.point)?;
    let residual = distance(&pa, &pb);
    if residual > floor {
        return Ok(None);
    }
    let contact = ss_contact(first, second, [u, v], [s, t], floor)?;
    let point = std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5);
    // Result control point must be finite before it enters a branch (1093).
    require_finite_point(&point, "ssi refined point")?;
    Ok(Some(([u, v], [s, t], point, contact)))
}

fn near_seed(existing: &[([f64; 2], [f64; 2])], uv: [f64; 2], st: [f64; 2], floor: f64) -> bool {
    existing.iter().any(|(a, b)| {
        (a[0] - uv[0]).abs() <= floor * 8.
            && (a[1] - uv[1]).abs() <= floor * 8.
            && (b[0] - st[0]).abs() <= floor * 8.
            && (b[1] - st[1]).abs() <= floor * 8.
    })
}

/// Certified general NURBS surface/surface intersection.
pub fn intersect_surface_surface_report(
    first: &Surface,
    second: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceSurfaceIntersection> {
    admit_surface(first)?;
    admit_surface(second)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let mut components = Vec::new();
    let mut unresolved = Vec::new();
    let mut boxes_visited = 0_usize;
    let mut bernstein_excluded = 0_usize;
    let mut krawczyk_isolated = 0_usize;
    let mut used_seeds: Vec<([f64; 2], [f64; 2])> = Vec::new();

    // Exact planar reduction when both charts are affine planes.
    if let Some(planar) = plane_plane_line(first, second, dist_floor)? {
        match planar {
            PlanarIntersection::Empty => {}
            PlanarIntersection::Overlap(region) => {
                components.push(SurfaceSurfaceComponent::Overlap(region))
            }
            PlanarIntersection::Curve(branch) => {
                components.push(SurfaceSurfaceComponent::Exact(branch))
            }
        }
        return Ok(SurfaceSurfaceIntersection {
            components,
            unresolved,
            boxes_visited: boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            tolerance,
        });
    }
    // Exact iso reduction for a freeform chart against an affine plane carrier.
    if let Some((n, o, _, _)) = affine_plane(second)?
        && let Some(iso) = surface_plane_iso_components(first, n, o, second, dist_floor)?
    {
        return Ok(SurfaceSurfaceIntersection {
            components: iso
                .into_iter()
                .map(SurfaceSurfaceComponent::Exact)
                .collect(),
            unresolved,
            boxes_visited: boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            tolerance,
        });
    }
    if let Some((n, o, _, _)) = affine_plane(first)?
        && let Some(mut iso) = surface_plane_iso_components(second, n, o, first, dist_floor)?
    {
        for component in &mut iso {
            component.swap_supports();
        }
        return Ok(SurfaceSurfaceIntersection {
            components: iso
                .into_iter()
                .map(SurfaceSurfaceComponent::Exact)
                .collect(),
            unresolved,
            boxes_visited: boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            tolerance,
        });
    }

    let cells_a = surface_spans(first)?;
    let cells_b = surface_spans(second)?;
    check(
        cells_a.len().saturating_mul(cells_b.len()) <= MAX_SPANS,
        "Surface span-pair resource exceeded",
    )?;

    let mut pending: std::collections::VecDeque<SurfaceCellPending> = cells_a
        .into_iter()
        .flat_map(|a| {
            cells_b
                .iter()
                .copied()
                .map(move |b| (a, b, None, None, 0_usize))
        })
        .collect();

    // Unified guard as runaway insurance (item 1065): total pops are bounded
    // by initial cells plus two children per counted box, so the guard only
    // fires if the bookkeeping above is ever broken. MAX_BOXES itself keeps
    // its existing unresolved-entry semantics.
    let mut guard = Budget::new(MAX_SPANS + 2 * MAX_BOXES + 1, 37, u64::MAX)?
        .guard("ssi_subdivision");
    while let Some((uv, st, ha, hb, depth)) = pending.pop_front() {
        guard.tick()?;
        guard.check()?;
        if boxes_visited >= MAX_BOXES {
            unresolved.push(UnresolvedSurfaceIntersection {
                parameter_box: Some([uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3]]),
                reason: UnresolvedReason::ResourceBoundary,
                classification: None,
            });
            continue;
        }
        boxes_visited += 1;
        let (ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (ha, hb),
            _ => {
                let pa = first.trim(uv)?;
                let pb = second.trim(st)?;
                (homogeneous_grid(&pa), homogeneous_grid(&pb))
            }
        };
        if grids_excluded_ss(&ha, &hb) {
            bernstein_excluded += 1;
            continue;
        }
        if proportional_grids(&ha, &hb, dist_floor) {
            components.push(SurfaceSurfaceComponent::Overlap(OverlapRegion {
                first_uv_box: uv,
                second_uv_box: st,
                geometry_enclosure: enclosure_of(
                    point3(
                        &first
                            .evaluate((uv[0] + uv[1]) * 0.5, (uv[2] + uv[3]) * 0.5)?
                            .point,
                    )?,
                    dist_floor,
                ),
                coedge_trim: coedge_trim([uv[0], uv[1]], [st[0], st[1]], [[0, 0], [0, 0]]),
            }));
            continue;
        }
        let width = (uv[1] - uv[0])
            .max(uv[3] - uv[2])
            .max(st[1] - st[0])
            .max(st[3] - st[2]);
        if width <= floor || depth >= 36 {
            let seed_uv = [(uv[0] + uv[1]) * 0.5, (uv[2] + uv[3]) * 0.5];
            let seed_st = [(st[0] + st[1]) * 0.5, (st[2] + st[3]) * 0.5];
            let pa = point3(&first.evaluate(seed_uv[0], seed_uv[1])?.point)?;
            let pb = point3(&second.evaluate(seed_st[0], seed_st[1])?.point)?;
            let residual = distance(&pa, &pb);
            let flat_a: Vec<[f64; 4]> = ha.iter().flatten().copied().collect();
            let flat_b: Vec<[f64; 4]> = hb.iter().flatten().copied().collect();
            let diag = next_up(hull_diagonal(&flat_a) + hull_diagonal(&flat_b));
            if residual - diag > dist_floor {
                continue;
            }
            match refine_seed(first, second, seed_uv, seed_st, uv, st, dist_floor)? {
                Some((ruv, rst, point, contact)) => {
                    if contact == ContactClass::PoleOrSingular {
                        unresolved.push(UnresolvedSurfaceIntersection {
                            parameter_box: Some([
                                uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3],
                            ]),
                            reason: UnresolvedReason::ConditioningBoundary,
                            classification: Some(UnresolvedClassification::PoleOrSingular),
                        });
                        continue;
                    }
                    if near_seed(&used_seeds, ruv, rst, floor) {
                        continue;
                    }
                    used_seeds.push((ruv, rst));
                    krawczyk_isolated += 1;
                    if matches!(
                        contact,
                        ContactClass::EvenTangency | ContactClass::OddTangency
                    ) {
                        components.push(SurfaceSurfaceComponent::Tangency(TangencyBranch {
                            contact,
                            sample: ContinuationSample {
                                point,
                                uv_first: ruv,
                                uv_second: rst,
                                parameter: 0.,
                                seam_wrap: None,
                            },
                            geometry_enclosure: enclosure_of(point, dist_floor),
                            coedge_trim: coedge_trim(
                                [ruv[0], ruv[0]],
                                [rst[0], rst[0]],
                                [[0, 0], [0, 0]],
                            ),
                            junction: false,
                        }));
                    } else {
                        let branch =
                            continue_branch(first, second, ruv, rst, point, contact, dist_floor)?;
                        components.push(SurfaceSurfaceComponent::Continued(branch));
                    }
                }
                None => {
                    if residual <= dist_floor {
                        unresolved.push(UnresolvedSurfaceIntersection {
                            parameter_box: Some([
                                uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3],
                            ]),
                            reason: UnresolvedReason::ConditioningBoundary,
                            classification: None,
                        });
                    } else {
                        unresolved.push(UnresolvedSurfaceIntersection {
                            parameter_box: Some([
                                uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3],
                            ]),
                            reason: UnresolvedReason::ConditioningBoundary,
                            classification: Some(
                                UnresolvedClassification::NearMissOrIllConditioned,
                            ),
                        });
                    }
                }
            }
            continue;
        }
        // Split the largest of the four axes.
        let widths = [uv[1] - uv[0], uv[3] - uv[2], st[1] - st[0], st[3] - st[2]];
        let axis = widths
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
        match axis {
            0 => {
                let mid = (uv[0] + uv[1]) * 0.5;
                let (l, r) = split_grid_u(&ha);
                pending.push_back((
                    [uv[0], mid, uv[2], uv[3]],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([mid, uv[1], uv[2], uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            1 => {
                let mid = (uv[2] + uv[3]) * 0.5;
                let (l, r) = split_grid_v(&ha);
                pending.push_back((
                    [uv[0], uv[1], uv[2], mid],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([uv[0], uv[1], mid, uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            2 => {
                let mid = (st[0] + st[1]) * 0.5;
                let (l, r) = split_grid_u(&hb);
                pending.push_back((
                    uv,
                    [st[0], mid, st[2], st[3]],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [mid, st[1], st[2], st[3]], Some(ha), Some(r), depth + 1));
            }
            _ => {
                let mid = (st[2] + st[3]) * 0.5;
                let (l, r) = split_grid_v(&hb);
                pending.push_back((
                    uv,
                    [st[0], st[1], st[2], mid],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [st[0], st[1], mid, st[3]], Some(ha), Some(r), depth + 1));
            }
        }
    }

    if components.len() > MAX_BRANCHES {
        unresolved.push(UnresolvedSurfaceIntersection {
            parameter_box: None,
            reason: UnresolvedReason::ResourceBoundary,
            classification: Some(UnresolvedClassification::BranchBudget),
        });
        components.truncate(MAX_BRANCHES);
    }

    // Detect junctions: shared endpoints among distinct curve components.
    mark_junctions(&mut components, floor);

    Ok(SurfaceSurfaceIntersection {
        components,
        unresolved,
        boxes_visited: boxes_visited,
        bernstein_excluded,
        krawczyk_isolated,
        tolerance,
    })
}

#[cfg(test)]
mod guard_tests {
    use super::*;
    use crate::foundation::guards::Budget;

    fn plane(z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 1., z]],
                vec![vec![1., 0., z], vec![1., 1., z]],
            ],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn nan_control_point_is_rejected_at_the_boundary() {
        let a = plane(0.);
        let mut b = plane(0.5);
        b.control_points[0][0][2] = f64::NAN;
        let err = intersect_surface_surface_report(&a, &b, None).err().expect("invalid intersection input must fail");
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    }

    #[test]
    fn crossing_planes_resolve_with_guards_active() {
        // Plane z=0 against the tilted plane z=x-0.5: one exact branch.
        let a = plane(0.);
        let mut b = plane(0.);
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[0] - 0.5;
                p[0] = 0.5;
            }
        }
        let r = intersect_surface_surface_report(&a, &b, None).unwrap();
        assert!(!r.components.is_empty());
    }

    #[test]
    fn marching_guard_matches_continuation_cap() {
        let mut guard =
            Budget::with_iterations(MAX_CONTINUATION).unwrap().guard("ssi_marching");
        for _ in 0..MAX_CONTINUATION {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("ssi_marching"));
    }
}
