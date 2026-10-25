//! Whole pair separation near a shared curved natural U boundary.
//! The exact common V curve glues two parameter rectangles continuously. A
//! fixed projection with a nonsingular Jacobian interval hull (or a validated
//! contraction matrix) proves joint injectivity on that glued rectangle.
//! Restricted control hulls independently
//! exclude both remaining outer strips, so no face portion is omitted.
use super::{I, Report, hull, section_jacobian};
use crate::{Result, check, surface::Surface};

fn eligible(s: &Surface) -> bool {
    !s.periodic_u
        && !s.periodic_v
        && s.degree_u <= 32
        && s.degree_v <= 32
        && s.control_points.len() == s.degree_u + 1
        && s.control_points[0].len() == s.degree_v + 1
        && s.control_points.iter().flatten().all(|p| p.len() == 3)
        && [(&s.knots_u, s.degree_u), (&s.knots_v, s.degree_v)]
            .into_iter()
            .all(|(knots, p)| {
                knots.len() == 2 * (p + 1)
                    && knots[..=p].iter().all(|&x| x == knots[p])
                    && knots[p + 1..].iter().all(|&x| x == knots[p + 1])
                    && knots[p] < knots[p + 1]
            })
}
// Reuse the same original Bernstein restriction already used by section
// Jacobians. General B-splines retain their original blossom fallback.
fn restricted_net(s: &Surface,domain: [[f64;2];2]) -> Result<Vec<Vec<[I;4]>>> {
    let span=[s.degree_u,s.degree_v];
    match crate::curve_surface_composition::surface_net_on_bernstein(s,span,domain)? {
        Some(net)=>Ok(net),
        None=>crate::curve_surface_composition::surface_net_on(s,span,domain),
    }
}
fn points(s: &Surface, domain: [[f64; 2]; 2]) -> Result<Vec<[I; 3]>> {
    restricted_net(s,domain)?
        .into_iter()
        .flatten()
        .map(|h| Ok([h[0].div(h[3])?, h[1].div(h[3])?, h[2].div(h[3])?]))
        .collect()
}
fn separated_along(a:&[[I;3]],b:&[[I;3]],directions:&[[f64;3]])->Result<bool> {
    for direction in directions {
        if direction.iter().any(|x| !x.is_finite()) {
            continue;
        }
        let project = |points: &[[I; 3]]| -> Result<I> {
            let mut result = Vec::new();
            for point in points {
                result.push(
                    point[0]
                        .mul(I::point(direction[0]))?
                        .add(point[1].mul(I::point(direction[1]))?)?
                        .add(point[2].mul(I::point(direction[2]))?)?,
                );
            }
            Ok(hull(result.into_iter()))
        };
        let x = project(a)?;
        let y = project(b)?;
        if x.hi < y.lo || y.hi < x.lo {
            return Ok(true);
        }
    }
    Ok(false)
}

fn separate(a: &[[I; 3]], b: &[[I; 3]]) -> Result<bool> {
    let bounds =
        |points: &[[I; 3]]| std::array::from_fn::<_, 3, _>(|k| hull(points.iter().map(|p| p[k])));
    let aa = bounds(a);
    let bb = bounds(b);
    let center = |bounds: [I; 3]| bounds.map(|v| v.lo * 0.5 + v.hi * 0.5);
    let ca = center(aa);
    let cb = center(bb);
    let mut directions = vec![[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    directions.push(std::array::from_fn(|k| ca[k] - cb[k]));
    if separated_along(a,b,&directions)? {return Ok(true);}
    if a.len() <= 64 && b.len() <= 64 {
        // Bounded numerical proposal for an oblique separating direction.
        // Midpoints never establish separation: every original interval pole
        // is subsequently projected and required to have a strict gap.
        let midpoint = |p: &[I; 3]| p.map(|v| v.lo * 0.5 + v.hi * 0.5);
        let aa = a.iter().map(midpoint).collect::<Vec<_>>();
        let bb = b.iter().map(midpoint).collect::<Vec<_>>();
        let dot = |x: [f64; 3], y: [f64; 3]| (0..3).map(|k| x[k] * y[k]).sum::<f64>();
        let mut normal = std::array::from_fn(|k| ca[k] - cb[k]);
        // Remove a long shared drift from the centroid proposal. Six bounded
        // support-axis choices also cover thin oblique parallel strips without
        // requiring convergence of the iterative proposal below.
        for cloud in [&aa, &bb] {
            for axis in 0..3 {
                let lo = cloud
                    .iter()
                    .min_by(|x, y| x[axis].total_cmp(&y[axis]))
                    .unwrap();
                let hi = cloud
                    .iter()
                    .max_by(|x, y| x[axis].total_cmp(&y[axis]))
                    .unwrap();
                let drift = std::array::from_fn(|k| hi[k] - lo[k]);
                let length = dot(drift, drift);
                if length > 0. && length.is_finite() {
                    let factor = dot(normal, drift) / length;
                    directions.push(std::array::from_fn(|k| normal[k] - factor * drift[k]));
                }
            }
        }
        if separated_along(a,b,&directions[4..])? {return Ok(true);}
        for _ in 0..64 {
            let first = aa
                .iter()
                .min_by(|x, y| dot(normal, **x).total_cmp(&dot(normal, **y)))
                .unwrap();
            let second = bb
                .iter()
                .max_by(|x, y| dot(normal, **x).total_cmp(&dot(normal, **y)))
                .unwrap();
            let mut direction = std::array::from_fn(|k| first[k] - second[k]);
            let length = dot(direction, direction).sqrt();
            if !length.is_finite() || length == 0. {
                break;
            }
            for v in &mut direction {
                *v /= length;
            }
            let margin = dot(normal, direction);
            if !margin.is_finite() || margin >= 1. {
                break;
            }
            for k in 0..3 {
                normal[k] += (1. - margin) * direction[k];
            }
        }
        directions.push(normal);
    }
    if directions.len()>4 {separated_along(a,b,&directions[directions.len()-1..])} else {Ok(false)}
}

// Closed subdivisions cover both complete V ranges independently. Every leaf
// requires a strict original interval-control-hull gap; no matching parameter
// assumption or sampled separation is used.
// Same-invocation reuse of restrictions of one immutable surface. Keys contain
// every original domain endpoint bit; entries never cross surfaces or calls.
// The fixed cap bounds memory, and every product cell is still charged normally.
fn cached_points(
    cache: &mut std::collections::BTreeMap<[u64;4], Vec<[I;3]>>,
    surface: &Surface,
    domain: [[f64;2];2],
) -> Result<Vec<[I;3]>> {
    let key=[domain[0][0].to_bits(),domain[0][1].to_bits(),domain[1][0].to_bits(),domain[1][1].to_bits()];
    if let Some(value)=cache.get(&key) {return Ok(value.clone());}
    let value=points(surface,domain)?;
    if cache.len()<64 {cache.insert(key,value.clone());}
    Ok(value)
}

fn separated_domains(
    a: &Surface, b: &Surface, domains: [[[f64;2];2];2], spans: &mut usize, max: usize,
) -> Result<bool> {
    let widths=[domains[0][1][1]-domains[0][1][0],domains[1][1][1]-domains[1][1][0]];
    let mut pending=vec![domains];
    let mut caches=[std::collections::BTreeMap::new(),std::collections::BTreeMap::new()];
    while let Some(domain)=pending.pop() {
        if max.saturating_sub(*spans)<2 {return Ok(false);}
        let aa=cached_points(&mut caches[0],a,domain[0])?;
        let bb=cached_points(&mut caches[1],b,domain[1])?;*spans+=2;
        if separate(&aa,&bb)? {continue;}
        let relative=[(domain[0][1][1]-domain[0][1][0])/widths[0],(domain[1][1][1]-domain[1][1][0])/widths[1]];
        let side=usize::from(relative[1]>relative[0]);let interval=domain[side][1];
        let mid=interval[0]*0.5+interval[1]*0.5;
        if !(interval[0]<mid && mid<interval[1]) {return Ok(false);}
        let mut left=domain;let mut right=domain;
        left[side][1][1]=mid;right[side][1][0]=mid;
        pending.push(right);pending.push(left);
    }
    Ok(true)
}

// A positive diagonal parameter metric preserves the contraction theorem.
// The midpoint inverse and metric are proposals only: both row norms below
// are bounded by outward interval operations on the original Jacobian hull.
fn weighted_contraction(j: [[I; 2]; 2]) -> Result<Option<f64>> {
    let midpoint = j.map(|row| row.map(|v| v.lo * 0.5 + v.hi * 0.5));
    let det = midpoint[0][0] * midpoint[1][1] - midpoint[0][1] * midpoint[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(None);
    }
    let inverse = [
        [midpoint[1][1] / det, -midpoint[0][1] / det],
        [-midpoint[1][0] / det, midpoint[0][0] / det],
    ];
    if inverse.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(None);
    }
    let mut residual = [[0.; 2]; 2];
    for i in 0..2 {
        for k in 0..2 {
            let product = I::point(inverse[i][0])
                .mul(j[0][k])?
                .add(I::point(inverse[i][1]).mul(j[1][k])?)?;
            let r = I::point(if i == k { 1. } else { 0. }).sub(product)?;
            residual[i][k] = r.lo.abs().max(r.hi.abs());
        }
    }
    contraction_norm(residual)
}

fn contraction_norm(residual: [[f64; 2]; 2]) -> Result<Option<f64>> {
    let balanced = residual[0][1].sqrt() / residual[1][0].sqrt();
    let mut upper = f64::INFINITY;
    for ratio in [1., balanced] {
        if !ratio.is_finite() || ratio <= 0. {
            continue;
        }
        let a = I::point(residual[0][0]).add(I::point(residual[0][1]).div(I::point(ratio))?)?;
        let b = I::point(residual[1][1]).add(I::point(residual[1][0]).mul(I::point(ratio))?)?;
        upper = upper.min(a.hi.max(b.hi));
    }
    Ok(upper.is_finite().then_some(upper))
}

fn jacobian_hull_nonsingular(j: [[I; 2]; 2]) -> Result<bool> {
    let determinant = j[0][0].mul(j[1][1])?.sub(j[0][1].mul(j[1][0])?)?;
    Ok(determinant.lo > 0. || determinant.hi < 0.)
}

fn spatial_contraction(j: [[I; 2]; 3]) -> Result<Option<f64>> {
    let midpoint = j.map(|row| row.map(|v| v.lo * 0.5 + v.hi * 0.5));
    let norms = std::array::from_fn::<_, 2, _>(|axis| {
        midpoint.iter().fold(0_f64, |sum, row| sum.hypot(row[axis]))
    });
    if norms.iter().any(|v| !v.is_finite() || *v == 0.) {
        return Ok(None);
    }
    let normalized = midpoint.map(|row| [row[0] / norms[0], row[1] / norms[1]]);
    let gram = std::array::from_fn::<_, 2, _>(|i| {
        std::array::from_fn::<_, 2, _>(|k| {
            normalized.iter().map(|row| row[i] * row[k]).sum::<f64>()
        })
    });
    let det = gram[0][0] * gram[1][1] - gram[0][1] * gram[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(None);
    }
    let inverse = [
        [gram[1][1] / det, -gram[0][1] / det],
        [-gram[1][0] / det, gram[0][0] / det],
    ];
    // A finite 2x3 left inverse is merely a proposal for Y. The actual bound
    // ||I - Y DS|| below includes all three original derivative coordinates.
    let projection = std::array::from_fn::<_, 2, _>(|i| {
        std::array::from_fn::<_, 3, _>(|k| {
            (inverse[i][0] * normalized[k][0] + inverse[i][1] * normalized[k][1]) / norms[i]
        })
    });
    if projection.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(None);
    }
    let mut residual = [[0.; 2]; 2];
    for i in 0..2 {
        for k in 0..2 {
            let mut product = I::point(0.);
            for axis in 0..3 {
                product = product.add(I::point(projection[i][axis]).mul(j[axis][k])?)?;
            }
            let r = I::point(if i == k { 1. } else { 0. }).sub(product)?;
            residual[i][k] = r.lo.abs().max(r.hi.abs());
        }
    }
    contraction_norm(residual)
}

fn joined_jacobian(s: &Surface, domain: [[f64; 2]; 2], end: bool) -> Result<[[I; 2]; 3]> {
    if !s.weights.iter().all(|row| row.iter().all(|w| *w == row[0])) {
        return section_jacobian(s, [s.degree_u, s.degree_v], domain);
    }
    let boundary = if end { s.degree_u } else { 0 };
    let mut net =
        restricted_net(s,domain)?;
    let origin = &s.control_points[boundary][0];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let mut result = super::jacobian_from_net(&net, [s.degree_u, s.degree_v], domain)?;
    // The same V-independent weights make dS/dV a positive rational convex
    // combination of row-wise polynomial V derivatives. Divide each original
    // homogeneous row difference by its own constant weight, rather than
    // losing the common W factor in the generic quotient enclosure.
    let width = I::point(domain[1][1]).sub(I::point(domain[1][0]))?;
    for k in 0..3 {
        let mut derivatives = Vec::new();
        for row in &net {
            for j in 0..s.degree_v {
                derivatives.push(
                    row[j + 1][k]
                        .sub(row[j][k])?
                        .mul(I::point(s.degree_v as f64))?
                        .div(width)?
                        .div(row[0][3])?,
                );
            }
        }
        result[k][1] = if derivatives.is_empty() {
            I::point(0.)
        } else {
            hull(derivatives.into_iter())
        };
    }
    // Original weights are independent of V, hence the shared boundary is a
    // polynomial V curve. Subtracting that same curve from every U control
    // row preserves dS/dU exactly. Restriction preserves this independence.
    // Every division/product uses original homogeneous coefficient intervals;
    // the V derivative of the translated curve is deliberately not used.
    let reference = net[boundary]
        .iter()
        .map(|h| Ok([h[0].div(h[3])?, h[1].div(h[3])?, h[2].div(h[3])?]))
        .collect::<Result<Vec<_>>>()?;
    for row in &mut net {
        let weight = row[0][3];
        for (j, h) in row.iter_mut().enumerate() {
            for k in 0..3 {
                h[k] = h[k].sub(weight.mul(reference[j][k])?)?;
            }
        }
    }
    let transverse = super::jacobian_from_net(&net, [s.degree_u, s.degree_v], domain)?;
    for k in 0..3 {
        result[k][0] = transverse[k][0];
    }
    Ok(result)
}

mod glued_monotone;

/// The original strip proof remains the fast path. A second sufficient proof
/// uses one fixed projection with positive symmetric Jacobian on both complete
/// half-rectangles. Exact boundary matching makes their glued map continuous.
pub fn certify_shared_natural_boundary(a:&Surface,b:&Surface,ends:[bool;2],max_cells:usize)->Result<Report>{
    a.validate()?;b.validate()?;
    if !eligible(a)||!eligible(b)||a.degree_v!=b.degree_v||a.knots_v!=b.knots_v||max_cells<6 {
        return certify_shared_natural_boundary_strips(a,b,ends,max_cells);
    }
    let ia=if ends[0]{a.degree_u}else{0};let ib=if ends[1]{b.degree_u}else{0};
    let aligned;
    let b=if a.control_points[ia]==b.control_points[ib]&&a.weights[ia]==b.weights[ib]{b}
    else if a.control_points[ia].iter().eq(b.control_points[ib].iter().rev())&&a.weights[ia].iter().eq(b.weights[ib].iter().rev()) {
        // Bezier V knots have only their original clamped endpoints. Reversing
        // poles and weights alone is an exact parameter permutation; no rounded
        // knot arithmetic or reconstructed world coordinates enter the proof.
        let mut copy=b.clone();for row in &mut copy.control_points{row.reverse();}for row in &mut copy.weights{row.reverse();}aligned=copy;&aligned
    }else{return certify_shared_natural_boundary_strips(a,b,ends,max_cells);};
    let mut report=certify_shared_natural_boundary_strips(a,b,ends,max_cells)?;
    if report.proven||max_cells.saturating_sub(report.spans)<2{return Ok(report);}
    let (proven,cells)=glued_monotone::certify(a,b,ends,max_cells-report.spans)?;
    report.spans+=cells;
    if proven {report.proven=true;report.projection=None;report.contraction_upper=None;report.reason="whole-shared-boundary-glued-monotone-projection";}
    Ok(report)
}

/// Certify that the two complete natural faces can meet only on their common
/// U boundary. `ends` chooses Umax (true) or Umin (false) in each face. Trim/world
/// edge ownership, individual face validity and shell admission remain separate.
fn certify_shared_natural_boundary_strips(
    a: &Surface,
    b: &Surface,
    ends: [bool; 2],
    max_cells: usize,
) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(max_cells <= 100000, "Invalid shared-boundary chart budget")?;
    let mut report = Report {
        proven: false,
        projection: None,
        linear_projection: None,
        projective_projection: None,
        polar_projection: None,
        polar_p_matrix: None,
        contraction_upper: None,
        spans: 0,
        reason: "shared-boundary-unproved",
    };
    if max_cells < 6 {
        report.reason = "work-limit";
        return Ok(report);
    }
    if !eligible(a) || !eligible(b) || a.degree_v != b.degree_v || a.knots_v != b.knots_v {
        return Ok(report);
    }
    let rows = [
        if ends[0] { a.degree_u } else { 0 },
        if ends[1] { b.degree_u } else { 0 },
    ];
    if a.control_points[rows[0]] != b.control_points[rows[1]]
        || a.weights[rows[0]] != b.weights[rows[1]]
    {
        return Ok(report);
    }
    let surfaces = [a, b];
    let mut full = [[[0.; 2]; 2]; 2];
    let mut near = full;
    let mut outer = full;
    for i in 0..2 {
        let s = surfaces[i];
        full[i] = [
            [s.knots_u[s.degree_u], s.knots_u[s.degree_u + 1]],
            [s.knots_v[s.degree_v], s.knots_v[s.degree_v + 1]],
        ];
        near[i] = full[i];
        outer[i] = full[i];
        let u = full[i][0];
        // Degree only proposes a strip width; all remaining portions are
        // independently checked. Conic cross jets benefit from a tighter hull.
        let width = if s.degree_u == 2 { 0.25 } else { 0.75 };
        let fraction = if ends[i] { 1. - width } else { width };
        let cut = u[0] * (1. - fraction) + u[1] * fraction;
        if !cut.is_finite() || !(u[0] < cut && cut < u[1]) {
            return Ok(report);
        }
        if ends[i] {
            near[i][0][0] = cut;
            outer[i][0][1] = cut;
        } else {
            near[i][0][1] = cut;
            outer[i][0][0] = cut;
        }
    }
    // OuterA x allB and allA x outerB, together with nearA x nearB,
    // cover the entire face product, including the restriction boundaries.
    let aa = points(a, outer[0])?;
    let bb = points(b, full[1])?;
    report.spans += 2;
    if !separate(&aa, &bb)? && (max_cells<=6 || !separated_domains(a,b,[outer[0],full[1]],&mut report.spans,max_cells-4)?) {
        report.reason = "first-outer-strip-separation-unproved";
        return Ok(report);
    }
    let aa = points(a, full[0])?;
    let bb = points(b, outer[1])?;
    report.spans += 2;
    if !separate(&aa, &bb)? && (max_cells<=6 || !separated_domains(a,b,[full[0],outer[1]],&mut report.spans,max_cells-2)?) {
        report.reason = "second-outer-strip-separation-unproved";
        return Ok(report);
    }
    let mut jac = [[I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 2]; 3];
    for i in 0..2 {
        let s = surfaces[i];
        let mut j = joined_jacobian(s, near[i], ends[i])?;
        // The A side maps [-1,0] toward its shared edge; B maps [0,1] away.
        let sign = if (i == 0) == ends[i] { 1. } else { -1. };
        let u_width = I::point(near[i][0][1])
            .sub(I::point(near[i][0][0]))?
            .mul(I::point(sign))?;
        let v_width = I::point(near[i][1][1]).sub(I::point(near[i][1][0]))?;
        for k in 0..3 {
            j[k][0] = j[k][0].mul(u_width)?;
            j[k][1] = j[k][1].mul(v_width)?;
            for axis in 0..2 {
                jac[k][axis] = hull([jac[k][axis], j[k][axis]].into_iter());
            }
        }
        report.spans += 1;
    }
    report.reason = "joint-projection-contraction-unproved";
    for axes in [[0, 1], [0, 2], [1, 2]] {
        // For any segment in the convex glued rectangle, the mean Jacobian
        // lies in this same interval hull. If every matrix in that hull is
        // nonsingular, equal projected images force equal parameters. This
        // uses the hull of all derivative entries, not a pointwise normal
        // determinant; continuity across the exact common V curve is required.
        if jacobian_hull_nonsingular([jac[axes[0]], jac[axes[1]]])? {
            report.proven = true;
            report.projection = Some(axes);
            report.contraction_upper = None;
            report.reason = "whole-shared-boundary-jacobian-hull";
            return Ok(report);
        }
        if let Some(q) = weighted_contraction([jac[axes[0]], jac[axes[1]]])? {
            report.contraction_upper = Some(
                report
                    .contraction_upper
                    .map_or(q, |previous| previous.min(q)),
            );
            if q >= 1. {
                continue;
            }
            report.proven = true;
            report.projection = Some(axes);
            report.contraction_upper = Some(q);
            report.reason = "whole-shared-boundary-joint-projection";
            return Ok(report);
        }
    }
    if let Some(q) = spatial_contraction(jac)? {
        report.contraction_upper = Some(
            report
                .contraction_upper
                .map_or(q, |previous| previous.min(q)),
        );
        if q < 1. {
            report.proven = true;
            report.reason = "whole-shared-boundary-spatial-contraction";
        }
    }
    Ok(report)
}

#[cfg(test)]
#[path = "shared_boundary/tests/tests_fixtures.rs"]
mod tests;
