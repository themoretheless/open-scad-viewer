//! Signed normal offsets of authored rational surfaces, for rolling-ball search.
//! Rectangle enclosures cover every admitted source parameter; numerical jets
//! only propose continuation steps. Neither API certifies an embedded offset,
//! contact with trimmed faces, G1/G2 continuity, or a fillet topology change.
use crate::{Result, check, distance_bounds::Interval as I, numeric, surface::Surface};

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub image: Option<[[f64; 2]; 3]>,
    pub unit_normals: Option<[[f64; 2]; 3]>,
    pub spans: usize,
    pub reason: &'static str,
}
fn square(x: I) -> Result<I> {
    let lo = if x.lo > 0. {
        x.lo
    } else if x.hi < 0. {
        -x.hi
    } else {
        0.
    };
    let hi = x.lo.abs().max(x.hi.abs());
    I::new((lo * lo).next_down().max(0.), (hi * hi).next_up())
}
struct NormalizedNormal {
    unit: [I; 3],
    length: I,
    scale: f64,
}
fn divide_scalar(value: I, scale: f64) -> Result<I> {
    I::new((value.lo / scale).next_down(), (value.hi / scale).next_up())
}
fn normalize(normal: [I; 3]) -> Result<Option<NormalizedNormal>> {
    let scale = normal
        .iter()
        .flat_map(|x| [x.lo.abs(), x.hi.abs()])
        .fold(0_f64, f64::max);
    if scale == 0. || normal.iter().all(|x| x.contains(0.)) {
        return Ok(None);
    }
    let mut n = [I::point(0.); 3];
    let mut length_squared = I::point(0.);
    for k in 0..3 {
        // Avoid an overflowing reciprocal for subnormal source normals.
        n[k] = divide_scalar(normal[k], scale)?;
        length_squared = length_squared.add(square(n[k])?)?;
    }
    if length_squared.lo <= 0. {
        return Ok(None);
    }
    let length = I::new(
        length_squared.lo.sqrt().next_down().max(0.),
        length_squared.hi.sqrt().next_up(),
    )?;
    if length.lo == 0. {
        return Ok(None);
    }
    for k in 0..3 {
        n[k] = n[k].div(length)?.intersect(-1., 1.)?;
    }
    Ok(Some(NormalizedNormal {
        unit: n,
        length,
        scale,
    }))
}
fn interval_cross(a: [I; 3], b: [I; 3]) -> Result<[I; 3]> {
    let mut out = [I::point(0.); 3];
    for k in 0..3 {
        out[k] = a[(k + 1) % 3]
            .mul(b[(k + 2) % 3])?
            .sub(a[(k + 2) % 3].mul(b[(k + 1) % 3])?)?;
    }
    Ok(out)
}
fn parameter_branches(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<Vec<[[f64; 2]; 2]>> {
    crate::normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let periodic = [s.periodic_u, s.periodic_v];
    let mut branches = vec![domain];
    for axis in 0..2 {
        let start = knots[axis][degrees[axis]];
        let end = knots[axis][counts[axis]];
        if periodic[axis] && domain[axis][1] == end && domain[axis][0] > start {
            let extra = branches
                .iter()
                .map(|d| {
                    let mut mapped = *d;
                    mapped[axis] = [start; 2];
                    mapped
                })
                .collect::<Vec<_>>();
            branches.extend(extra);
        }
    }
    Ok(branches)
}
fn union_boxes(a: &mut [[f64; 2]; 3], b: [[f64; 2]; 3]) {
    for k in 0..3 {
        a[k][0] = a[k][0].min(b[k][0]);
        a[k][1] = a[k][1].max(b[k][1]);
    }
}
/// Conservative image of S(u,v) + distance * normalized(S_u cross S_v).
/// Includes all incident span sides and wrapped periodic endpoint parameters.
/// This does not prove a unique normal at a repeated knot. A missing image
/// means normal enclosure or span coverage is unproven; callers retain it.
pub fn bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    distance: f64,
    max_spans: usize,
) -> Result<Bounds> {
    check(
        distance.is_finite(),
        "Surface offset distance must be finite",
    )?;
    let branches = parameter_branches(s, domain, max_spans)?;
    let mut out = bounds_on(s, branches[0], distance, max_spans)?;
    for branch in branches.into_iter().skip(1) {
        if out.image.is_none() {
            return Ok(out);
        }
        if out.spans == max_spans {
            out.image = None;
            out.unit_normals = None;
            out.reason = "span-limit";
            return Ok(out);
        }
        let other = bounds_on(s, branch, distance, max_spans - out.spans)?;
        out.spans += other.spans;
        let Some(image) = other.image else {
            out.image = None;
            out.unit_normals = None;
            out.reason = other.reason;
            return Ok(out);
        };
        union_boxes(out.image.as_mut().unwrap(), image);
        if let (Some(a), Some(b)) = (&mut out.unit_normals, other.unit_normals) {
            union_boxes(a, b);
        }
    }
    Ok(out)
}
fn bounds_on(
    s: &Surface,
    domain: [[f64; 2]; 2],
    distance: f64,
    max_spans: usize,
) -> Result<Bounds> {
    check(
        distance.is_finite(),
        "Surface offset distance must be finite",
    )?;
    let (normal, spans) = crate::normal_alignment::normal_bounds(s, domain, max_spans)?;
    let mut out = Bounds {
        image: None,
        unit_normals: None,
        spans,
        reason: "span-limit",
    };
    let Some(normal) = normal else {
        return Ok(out);
    };
    let image = crate::surface_distance::rectangle_bounds(s, domain)?;
    if distance == 0. {
        out.image = Some(std::array::from_fn(|k| image[k]));
        out.reason = "zero-offset";
        return Ok(out);
    }
    out.reason = "source-normal-unresolved";
    let Some(normalized) = normalize(normal.map(|[lo, hi]| I { lo, hi }))? else {
        return Ok(out);
    };
    let mut output = [[0.; 2]; 3];
    let mut normals = [[0.; 2]; 3];
    for k in 0..3 {
        let unit = normalized.unit[k];
        let value = I::new(image[k][0], image[k][1])?.add(unit.mul(I::point(distance))?)?;
        output[k] = [value.lo, value.hi];
        normals[k] = [unit.lo, unit.hi];
    }
    out.image = Some(output);
    out.unit_normals = Some(normals);
    out.reason = "source-normal-offset-enclosure";
    Ok(out)
}
#[derive(Debug)]
pub struct JacobianBounds {
    pub image: Option<[[f64; 2]; 3]>,
    pub derivatives: Option<[[[f64; 2]; 3]; 2]>,
    pub spans: usize,
    pub reason: &'static str,
}
/// Encloses offset points and first derivatives on all incident span sides.
/// This does not certify continuity across knots or regularity of the offset.
pub fn jacobian_bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    distance: f64,
    max_spans: usize,
) -> Result<JacobianBounds> {
    check(
        distance.is_finite(),
        "Surface offset distance must be finite",
    )?;
    let branches = parameter_branches(s, domain, max_spans)?;
    let mut out = jacobian_bounds_on(s, branches[0], distance, max_spans)?;
    for branch in branches.into_iter().skip(1) {
        if out.image.is_none() {
            return Ok(out);
        }
        if out.spans == max_spans {
            out.image = None;
            out.derivatives = None;
            out.reason = "span-limit";
            return Ok(out);
        }
        let other = jacobian_bounds_on(s, branch, distance, max_spans - out.spans)?;
        out.spans += other.spans;
        let Some(image) = other.image else {
            out.image = None;
            out.derivatives = None;
            out.reason = other.reason;
            return Ok(out);
        };
        union_boxes(out.image.as_mut().unwrap(), image);
        for axis in 0..2 {
            union_boxes(
                &mut out.derivatives.as_mut().unwrap()[axis],
                other.derivatives.unwrap()[axis],
            );
        }
    }
    Ok(out)
}
fn jacobian_bounds_on(
    s: &Surface,
    domain: [[f64; 2]; 2],
    distance: f64,
    max_spans: usize,
) -> Result<JacobianBounds> {
    check(
        distance.is_finite(),
        "Surface offset distance must be finite",
    )?;
    let (source, spans) = crate::normal_alignment::jet_bounds(s, domain, max_spans)?;
    let mut out = JacobianBounds {
        image: None,
        derivatives: None,
        spans,
        reason: "span-limit",
    };
    let Some(source) = source else {
        return Ok(out);
    };
    let mut image = source.point;
    let mut derivatives = source.first;
    if distance != 0. {
        out.reason = "source-normal-unresolved";
        let raw = interval_cross(source.first[0], source.first[1])?;
        let Some(normal) = normalize(raw)? else {
            return Ok(out);
        };
        for axis in 0..2 {
            let a = interval_cross(source.second[axis], source.first[1])?;
            let b = interval_cross(source.first[0], source.second[axis + 1])?;
            let mut dn = [I::point(0.); 3];
            let mut parallel = I::point(0.);
            for k in 0..3 {
                dn[k] = divide_scalar(a[k].add(b[k])?, normal.scale)?;
                parallel = parallel.add(normal.unit[k].mul(dn[k])?)?;
            }
            for k in 0..3 {
                let unit_derivative = dn[k]
                    .sub(normal.unit[k].mul(parallel)?)?
                    .div(normal.length)?;
                derivatives[axis][k] =
                    derivatives[axis][k].add(unit_derivative.mul(I::point(distance))?)?;
            }
        }
        for k in 0..3 {
            image[k] = image[k].add(normal.unit[k].mul(I::point(distance))?)?;
        }
    }
    out.image = Some(image.map(|x| [x.lo, x.hi]));
    out.derivatives = Some(derivatives.map(|row| row.map(|x| [x.lo, x.hi])));
    out.reason = "offset-incident-span-jets";
    Ok(out)
}
#[derive(Debug)]
pub struct Evaluation {
    pub point: [f64; 3],
    pub du: [f64; 3],
    pub dv: [f64; 3],
    pub source_unit_normal: [f64; 3],
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
/// Numerical offset point and first derivatives for predictor/corrector seeds.
/// Discontinuous source jets are refused. A zero offset Jacobian is returned
/// unchanged; a point evaluation never certifies offset regularity.
pub fn evaluate(s: &Surface, uv: [f64; 2], distance: f64) -> Result<Evaluation> {
    check(
        distance.is_finite(),
        "Surface offset distance must be finite",
    )?;
    let sample = s.evaluate(uv[0], uv[1])?;
    let (du, dv) = sample
        .first_derivatives()
        .ok_or_else(|| crate::input("Surface offset needs defined source first derivatives"))?;
    let (duu, duv, dvv) = sample
        .second_derivatives()
        .ok_or_else(|| crate::input("Surface offset needs defined source second derivatives"))?;
    let normal = cross(du, dv);
    let length = normal[0].hypot(normal[1]).hypot(normal[2]);
    numeric(
        length > 0. && length.is_finite(),
        "Surface offset source normal is singular or out of range",
    )?;
    let n = normal.map(|x| x / length);
    let normal_u_a = cross(duu, dv);
    let normal_u_b = cross(du, duv);
    let normal_v_a = cross(duv, dv);
    let normal_v_b = cross(du, dvv);
    let nu = std::array::from_fn(|k| normal_u_a[k] + normal_u_b[k]);
    let nv = std::array::from_fn(|k| normal_v_a[k] + normal_v_b[k]);
    let normal_derivative = |d: [f64; 3]| {
        let parallel = dot(n, d);
        std::array::from_fn::<_, 3, _>(|k| (d[k] - n[k] * parallel) / length)
    };
    let nu = normal_derivative(nu);
    let nv = normal_derivative(nv);
    let out = Evaluation {
        point: std::array::from_fn(|k| sample.point[k] + distance * n[k]),
        du: std::array::from_fn(|k| du[k] + distance * nu[k]),
        dv: std::array::from_fn(|k| dv[k] + distance * nv[k]),
        source_unit_normal: n,
    };
    numeric(
        out.point
            .iter()
            .chain(&out.du)
            .chain(&out.dv)
            .all(|x| x.is_finite()),
        "Surface offset jets exceeded numeric range",
    )?;
    Ok(out)
}

fn source_continuous_for_offset(s: &Surface, domain: [[f64; 2]; 2]) -> bool {
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let periodic = [s.periodic_u, s.periodic_v];
    for axis in 0..2 {
        let start = knots[axis][degrees[axis]];
        let end = knots[axis][counts[axis]];
        // No periodic flag alone proves matching source jets across its seam.
        if periodic[axis] && domain[axis][1] == end {
            return false;
        }
        let mut i = 0;
        while i < knots[axis].len() {
            let value = knots[axis][i];
            let mut next = i + 1;
            while next < knots[axis].len() && knots[axis][next] == value {
                next += 1;
            }
            if value > start
                && value < end
                && value >= domain[axis][0]
                && value <= domain[axis][1]
                && next - i >= degrees[axis]
            {
                return false;
            }
            i = next;
        }
    }
    true
}
/// Unique offset/offset contact in a three-dimensional parameter section.
/// Witness.point encloses the shared offset center, not a point on an original
/// support. Witness UVs refer to the original supports. No trim membership,
/// whole-curve completeness, envelope patch or fillet topology is certified.
pub fn certify_contact_section(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed: f64,
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<crate::surface_contact::Verdict> {
    use crate::surface_contact::{SectionVerdict, Verdict, Witness, section_krawczyk};
    check(
        fixed_axis < 2 && fixed.is_finite(),
        "Choose a finite fixed offset parameter",
    )?;
    check(
        std::iter::once(&first_other)
            .chain(&second)
            .all(|d| d.iter().all(|x| x.is_finite()) && d[0] < d[1]),
        "Offset contact intervals must have positive finite widths",
    )?;
    let free = 1 - fixed_axis;
    let mut first = [first_other; 2];
    first[fixed_axis] = [fixed; 2];
    let initial = [
        bounds(surfaces[0], first, distances[0], max_spans)?,
        bounds(surfaces[1], second, distances[1], max_spans)?,
    ];
    let (Some(pa), Some(pb)) = (initial[0].image, initial[1].image) else {
        return Ok(Verdict::Unresolved);
    };
    if (0..3).any(|k| pa[k][1] < pb[k][0] || pb[k][1] < pa[k][0]) {
        return Ok(Verdict::Excluded);
    }
    if !source_continuous_for_offset(surfaces[0], first)
        || !source_continuous_for_offset(surfaces[1], second)
    {
        return Ok(Verdict::Unresolved);
    }
    let a = jacobian_bounds(surfaces[0], first, distances[0], max_spans)?;
    let b = jacobian_bounds(surfaces[1], second, distances[1], max_spans)?;
    let (Some(a), Some(b)) = (a.derivatives, b.derivatives) else {
        return Ok(Verdict::Unresolved);
    };
    let mut jac = [[I::point(0.); 3]; 3];
    for k in 0..3 {
        jac[k] = [
            I::new(a[free][k][0], a[free][k][1])?,
            I::point(0.).sub(I::new(b[0][k][0], b[0][k][1])?)?,
            I::point(0.).sub(I::new(b[1][k][0], b[1][k][1])?)?,
        ];
    }
    let domain = [first_other, second[0], second[1]];
    let center = domain.map(|r| r[0] * 0.5 + r[1] * 0.5);
    if (0..3).any(|k| center[k] <= domain[k][0] || center[k] >= domain[k][1]) {
        return Ok(Verdict::Unresolved);
    }
    let mut first_center = first;
    first_center[free] = [center[0]; 2];
    let ac = bounds(surfaces[0], first_center, distances[0], max_spans)?;
    let bc = bounds(
        surfaces[1],
        [[center[1]; 2], [center[2]; 2]],
        distances[1],
        max_spans,
    )?;
    let (Some(ac), Some(bc)) = (ac.image, bc.image) else {
        return Ok(Verdict::Unresolved);
    };
    let mut residual = [I::point(0.); 3];
    for k in 0..3 {
        residual[k] = I::new(ac[k][0], ac[k][1])?.sub(I::new(bc[k][0], bc[k][1])?)?;
    }
    let (parameters, contraction_upper) = match section_krawczyk(domain, jac, residual)? {
        SectionVerdict::Excluded => return Ok(Verdict::Excluded),
        SectionVerdict::Unresolved => return Ok(Verdict::Unresolved),
        SectionVerdict::Unique {
            parameters,
            contraction_upper,
        } => (parameters, contraction_upper),
    };
    first[free] = parameters[0];
    let second = [parameters[1], parameters[2]];
    let a = bounds(surfaces[0], first, distances[0], max_spans)?.image;
    let b = bounds(surfaces[1], second, distances[1], max_spans)?.image;
    let (Some(a), Some(b)) = (a, b) else {
        return Ok(Verdict::Unresolved);
    };
    let mut point = [[0.; 2]; 3];
    for k in 0..3 {
        let overlap = I::new(a[k][0].max(b[k][0]), a[k][1].min(b[k][1]))?;
        point[k] = [overlap.lo, overlap.hi];
    }
    Ok(Verdict::Witness(Witness {
        first_uv: first,
        second_uv: second,
        point,
        contraction_upper,
    }))
}

pub type PairDomain = [[[f64; 2]; 2]; 2];
#[derive(Debug)]
pub struct Candidates {
    pub boxes: Vec<PairDomain>,
    pub pending: Vec<PairDomain>,
    pub visited_boxes: usize,
    pub excluded_boxes: usize,
    pub normal_span_visits: usize,
    pub reason: &'static str,
}
/// Complete-domain exclusion stage for two signed normal-offset carriers.
/// Candidate boxes are possible center locations, never certified roots.
/// Pending boxes retain every unvisited, singular or precision-limited region.
/// UV trim admission and continuation/branch uniqueness belong to later stages.
pub fn intersection_candidates(
    surfaces: [&Surface; 2],
    domains: PairDomain,
    distances: [f64; 2],
    parameter_tolerance: f64,
    max_boxes: usize,
    max_spans_per_box: usize,
) -> Result<Candidates> {
    check(
        parameter_tolerance.is_finite() && parameter_tolerance > 0. && parameter_tolerance <= 1.,
        "Offset search parameter tolerance must lie in (0,1]",
    )?;
    check(
        (1..=100000).contains(&max_boxes),
        "Offset search needs 1..100000 boxes",
    )?;
    // Evaluate both full rectangles before an early exclusion can hide an
    // invalid second operand. Reuse these reports for the first visited box.
    let initial = [
        bounds(surfaces[0], domains[0], distances[0], max_spans_per_box)?,
        bounds(surfaces[1], domains[1], distances[1], max_spans_per_box)?,
    ];
    let widths = domains.map(|d| d.map(|r| r[1] - r[0]));
    check(
        widths.iter().flatten().all(|w| w.is_finite()),
        "Offset search parameter widths exceeded numeric range",
    )?;
    let mut queue = vec![domains];
    let mut out = Candidates {
        boxes: Vec::new(),
        pending: Vec::new(),
        visited_boxes: 0,
        excluded_boxes: 0,
        normal_span_visits: 0,
        reason: "candidate-boxes",
    };
    while let Some(cell) = queue.pop() {
        if out.visited_boxes == max_boxes {
            out.pending.push(cell);
            out.pending.extend(queue);
            out.reason = "work-limit";
            return Ok(out);
        }
        out.visited_boxes += 1;
        let mut images = [None, None];
        for side in 0..2 {
            let bound = if out.visited_boxes == 1 {
                initial[side]
            } else {
                bounds(
                    surfaces[side],
                    cell[side],
                    distances[side],
                    max_spans_per_box,
                )?
            };
            out.normal_span_visits += bound.spans;
            images[side] = bound.image;
        }
        if let [Some(a), Some(b)] = images {
            if crate::surface_distance::enclosure_distance(&a, &b)?.0 > 0. {
                out.excluded_boxes += 1;
                continue;
            }
        }
        let (side, axis, width) = (0..2)
            .flat_map(|side| (0..2).map(move |axis| (side, axis)))
            .map(|(side, axis)| {
                let width = if widths[side][axis] == 0. {
                    0.
                } else {
                    (cell[side][axis][1] - cell[side][axis][0]) / widths[side][axis]
                };
                (side, axis, width)
            })
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .unwrap();
        if width <= parameter_tolerance {
            if images.iter().all(Option::is_some) {
                out.boxes.push(cell);
            } else {
                out.pending.push(cell);
                out.reason = "source-normal-unresolved";
            }
            continue;
        }
        let [lo, hi] = cell[side][axis];
        let mid = lo * 0.5 + hi * 0.5;
        if mid <= lo || mid >= hi {
            out.pending.push(cell);
            out.reason = "precision-limit";
            continue;
        }
        for range in [[lo, mid], [mid, hi]] {
            let mut child = cell;
            child[side][axis] = range;
            queue.push(child);
        }
    }
    if out.pending.is_empty() && out.boxes.is_empty() {
        out.reason = "all-excluded";
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn folded_periodic_plane() -> Surface {
        let mut s = plane();
        s.periodic_u = true;
        s.knots_u = vec![-1., 0., 1., 2., 3.];
        s.control_points.push(s.control_points[0].clone());
        s.weights.push(s.weights[0].clone());
        s
    }
    fn cylinder() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![[3., 0.], [3., 3.], [0., 3.]]
                .into_iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
                .collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
                .into_iter()
                .map(|w| vec![w; 2])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn enclosed(image: [[f64; 2]; 3], point: [f64; 3]) {
        for k in 0..3 {
            assert!(
                image[k][0] <= point[k] && point[k] <= image[k][1],
                "{image:?} misses {point:?}"
            );
        }
    }
    #[test]
    fn signed_plane_offsets_and_jets_preserve_authored_geometry() {
        let s = plane();
        let before = s.clone();
        for distance in [-2., 0., 2.] {
            let r = bounds(&s, [[0., 1.], [0., 1.]], distance, 1).unwrap();
            enclosed(r.image.unwrap(), [0., 0., distance]);
            enclosed(r.image.unwrap(), [1., 1., distance]);
            let e = evaluate(&s, [0.37, 0.62], distance).unwrap();
            assert_eq!(e.point, [0.37, 0.62, distance]);
            assert_eq!(e.du, [1., 0., 0.]);
            assert_eq!(e.dv, [0., 1., 0.]);
        }
        assert_eq!(s, before);
    }
    #[test]
    fn rational_cylinder_offsets_cover_complete_parameter_cells() {
        let s = cylinder();
        for distance in [-2., 1.5] {
            for i in 0..16 {
                let lo = i as f64 / 16.;
                let hi = (i + 1) as f64 / 16.;
                let r = bounds(&s, [[lo, hi], [0., 1.]], distance, 1).unwrap();
                assert!(r.image.is_some(), "{r:?}");
                let jacobian = jacobian_bounds(&s, [[lo, hi], [0., 1.]], distance, 1).unwrap();
                assert!(jacobian.derivatives.is_some(), "{jacobian:?}");
                for u in [lo, (lo + hi) / 2., hi] {
                    let e = evaluate(&s, [u, 0.37], distance).unwrap();
                    enclosed(r.image.unwrap(), e.point);
                    enclosed(jacobian.image.unwrap(), e.point);
                    for axis in 0..2 {
                        enclosed(jacobian.derivatives.unwrap()[axis], [e.du, e.dv][axis]);
                    }
                    assert!((e.point[0].hypot(e.point[1]) - (3. + distance)).abs() < 1e-12);
                    assert!((e.point[2] - 1.85).abs() < 1e-12);
                    let h = 1e-5;
                    if u > h && u < 1. - h {
                        let left = evaluate(&s, [u - h, 0.37], distance).unwrap();
                        let right = evaluate(&s, [u + h, 0.37], distance).unwrap();
                        for k in 0..3 {
                            assert!(
                                ((right.point[k] - left.point[k]) / (2. * h) - e.du[k]).abs()
                                    < 1e-8
                            );
                        }
                    }
                }
            }
        }
        // The inward radius-three offset collapses to the cylinder axis.
        // An image enclosure is not a proof of a regular offset surface.
        let collapsed = evaluate(&s, [0.37, 0.62], -3.).unwrap();
        assert!(collapsed.du.iter().all(|x| x.abs() < 1e-12));
    }
    #[test]
    fn reversing_a_surface_parameter_requires_reversing_the_signed_offset() {
        let s = cylinder();
        let reversed = s
            .edit_axis(crate::surface::Axis::U, |c| c.reverse())
            .unwrap();
        let a = evaluate(&s, [0.37, 0.62], 1.5).unwrap();
        let b = evaluate(&reversed, [0.63, 0.62], -1.5).unwrap();
        for k in 0..3 {
            assert!((a.point[k] - b.point[k]).abs() < 1e-12);
            assert!((a.du[k] + b.du[k]).abs() < 1e-12);
            assert!((a.dv[k] - b.dv[k]).abs() < 1e-12);
        }
        let r = bounds(&reversed, [[0.6, 0.65], [0.6, 0.65]], -1.5, 1).unwrap();
        enclosed(r.image.unwrap(), b.point);
    }
    #[test]
    fn general_spatial_rational_patch_offset_bounds_and_jets() {
        let s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            let x = i as f64 / 2.;
                            let y = j as f64 / 2.;
                            let z = 0.2 * x * y + 0.1 * x * x;
                            vec![17. + z, -9. + x, 23. + y]
                        })
                        .collect()
                })
                .collect(),
            weights: (0..3)
                .map(|i| (0..3).map(|j| 1. + (i + j) as f64 / 5.).collect())
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        for distance in [-0.7, 0.7] {
            for i in 0..8 {
                for j in 0..8 {
                    let u = [i as f64 / 8., (i + 1) as f64 / 8.];
                    let v = [j as f64 / 8., (j + 1) as f64 / 8.];
                    let r = bounds(&s, [u, v], distance, 1).unwrap();
                    let uv = [(u[0] + u[1]) / 2., (v[0] + v[1]) / 2.];
                    let e = evaluate(&s, uv, distance).unwrap();
                    enclosed(r.image.unwrap(), e.point);
                    let jacobian = jacobian_bounds(&s, [u, v], distance, 1).unwrap();
                    enclosed(jacobian.image.unwrap(), e.point);
                    enclosed(jacobian.derivatives.unwrap()[0], e.du);
                    enclosed(jacobian.derivatives.unwrap()[1], e.dv);
                    for axis in 0..2 {
                        let mut left = uv;
                        let mut right = uv;
                        left[axis] -= 1e-5;
                        right[axis] += 1e-5;
                        let left = evaluate(&s, left, distance).unwrap();
                        let right = evaluate(&s, right, distance).unwrap();
                        for k in 0..3 {
                            let derivative = [e.du, e.dv][axis][k];
                            assert!(
                                ((right.point[k] - left.point[k]) / 2e-5 - derivative).abs() < 1e-7
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(s, before);
    }
    #[test]
    fn subnormal_regular_source_normals_do_not_require_an_overflowing_reciprocal() {
        let mut s = plane();
        for row in &mut s.control_points {
            for p in row {
                p[0] *= 1e-160;
                p[1] *= 1e-160;
            }
        }
        let r = bounds(&s, [[0., 1.], [0., 1.]], 0.2, 1).unwrap();
        enclosed(r.image.unwrap(), [0., 0., 0.2]);
        enclosed(r.image.unwrap(), [1e-160, 1e-160, 0.2]);
    }
    #[test]
    fn periodic_endpoints_include_the_wrapped_parameter_branch() {
        let s = folded_periodic_plane();
        let d = [[2., 2.], [0.3, 0.3]];
        assert_eq!(s.evaluate(2., 0.3).unwrap().point[0], 0.);
        // The degree-one seam has two different limiting normals; a numerical
        // regular offset jet there is refused. Bounds retain both span sides.
        assert!(evaluate(&s, [2., 0.3], 0.2).is_err());
        let r = bounds(&s, d, 0.2, 2).unwrap();
        enclosed(r.image.unwrap(), [0., 0.3, -0.2]);
        enclosed(r.image.unwrap(), [0., 0.3, 0.2]);
        assert_eq!(r.spans, 2);
        assert!(bounds(&s, d, 0.2, 1).unwrap().image.is_none());
        let j = jacobian_bounds(&s, d, 0.2, 2).unwrap();
        enclosed(j.image.unwrap(), [0., 0.3, -0.2]);
        enclosed(j.image.unwrap(), [0., 0.3, 0.2]);
        enclosed(j.derivatives.unwrap()[0], [1., 0., 0.]);
        enclosed(j.derivatives.unwrap()[0], [-1., 0., 0.]);
        assert!(jacobian_bounds(&s, d, 0.2, 1).unwrap().image.is_none());
    }
    #[test]
    fn crossing_offset_planes_have_a_unique_section_center() {
        use crate::surface_contact::Verdict;
        let a = plane();
        let mut b = a.clone();
        // B is the XZ plane at Y=0.5, with normal pointing toward -Y.
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z;
            }
        }
        let r = certify_contact_section(
            [&a, &b],
            [0.2, 0.2],
            0,
            0.37,
            [0.25, 0.35],
            [[0.32, 0.42], [0.15, 0.25]],
            2,
        )
        .unwrap();
        let Verdict::Witness(w) = r else {
            panic!("{r:?}");
        };
        enclosed(w.point, [0.37, 0.3, 0.2]);
        assert!(w.contraction_upper < 0.5);
        assert!(w.first_uv[1][0] <= 0.3 && w.first_uv[1][1] >= 0.3);
        let coincident = certify_contact_section(
            [&a, &a],
            [0.2, 0.2],
            0,
            0.37,
            [0.25, 0.35],
            [[0.32, 0.42], [0.25, 0.35]],
            2,
        )
        .unwrap();
        assert!(matches!(coincident, Verdict::Unresolved));
        let separated = certify_contact_section(
            [&a, &b],
            [0.2, 0.2],
            0,
            0.37,
            [0.75, 0.85],
            [[0.32, 0.42], [0.15, 0.25]],
            2,
        )
        .unwrap();
        assert!(matches!(separated, Verdict::Excluded));
    }
    #[test]
    fn rational_cylinder_plane_offsets_have_a_certified_contact_section() {
        use crate::surface_contact::Verdict;
        let a = cylinder();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                let y = 4. * p[0];
                let z = 5. * p[1];
                *p = vec![2., y, z];
            }
        }
        // Numerical Newton proposes the interval only; inclusion below is
        // established independently from the original interval source jets.
        let mut u = 0.5;
        for _ in 0..8 {
            let e = evaluate(&a, [u, 0.37], 0.2).unwrap();
            u -= (e.point[0] - 2.2) / e.du[0];
        }
        let y = (3.2_f64 * 3.2 - 2.2 * 2.2).sqrt();
        let b_u = y / 4.;
        let r = certify_contact_section(
            [&a, &b],
            [0.2, 0.2],
            1,
            0.37,
            [u - 1e-4, u + 1e-4],
            [[b_u - 1e-4, b_u + 1e-4], [0.3699, 0.3701]],
            2,
        )
        .unwrap();
        let Verdict::Witness(w) = r else {
            panic!("{r:?}");
        };
        enclosed(w.point, [2.2, y, 1.85]);
        assert!(w.contraction_upper < 0.5);
        assert!(w.point.iter().all(|p| p[1] - p[0] < 1e-5));
        for side in 0..2 {
            let uv = [w.first_uv, w.second_uv][side].map(|r| (r[0] + r[1]) / 2.);
            let sample = evaluate([&a, &b][side], uv, 0.2).unwrap();
            assert!((sample.point[0] - 2.2).abs() < 1e-5);
            assert!((sample.point[1] - y).abs() < 1e-5);
        }
    }
    #[test]
    fn collapsed_offset_carrier_cannot_certify_a_unique_contact() {
        use crate::surface_contact::Verdict;
        let a = cylinder();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                *p = vec![2. * p[0] - 1., 2. * p[1] - 1., 1.85];
            }
        }
        let r = certify_contact_section(
            [&a, &b],
            [-3., 0.],
            1,
            0.37,
            [0.36, 0.38],
            [[0.49, 0.51], [0.49, 0.51]],
            2,
        )
        .unwrap();
        assert!(matches!(r, Verdict::Unresolved));
    }
    #[test]
    fn offset_contact_does_not_infer_continuity_from_a_periodic_flag() {
        use crate::surface_contact::Verdict;
        let a = folded_periodic_plane();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z;
            }
        }
        let r = certify_contact_section(
            [&a, &b],
            [0.2, 0.2],
            0,
            2.,
            [0.25, 0.35],
            [[0., 0.1], [0.15, 0.25]],
            2,
        )
        .unwrap();
        assert!(matches!(r, Verdict::Unresolved));
    }
    #[test]
    fn offset_jacobian_keeps_both_incident_knot_sides_and_budget_limits() {
        let mut s = plane();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points
            .insert(1, vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]]);
        s.weights.insert(1, vec![1.; 2]);
        let point = [[0.5, 0.5], [0.3, 0.3]];
        let r = jacobian_bounds(&s, point, 1., 2).unwrap();
        assert_eq!(r.spans, 2);
        // Source U derivative jumps from 0.5 to 1.5; both sides must be enclosed.
        enclosed(r.derivatives.unwrap()[0], [0.5, 0., 0.]);
        enclosed(r.derivatives.unwrap()[0], [1.5, 0., 0.]);
        assert!(
            jacobian_bounds(&s, point, 1., 1)
                .unwrap()
                .derivatives
                .is_none()
        );
        let mut singular = plane();
        singular.control_points[1] = singular.control_points[0].clone();
        assert!(
            jacobian_bounds(&singular, [[0., 1.], [0., 1.]], 1., 1)
                .unwrap()
                .derivatives
                .is_none()
        );
    }
    #[test]
    fn singular_source_and_budget_exhaustion_keep_offset_image_unproven() {
        let mut singular = plane();
        singular.control_points[1] = singular.control_points[0].clone();
        assert!(
            bounds(&singular, [[0., 1.], [0., 1.]], 1., 1)
                .unwrap()
                .image
                .is_none()
        );
        assert!(evaluate(&singular, [0.4, 0.6], 1.).is_err());
        let mut s = plane();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points
            .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
        s.weights.insert(1, vec![1.; 2]);
        let incomplete = bounds(&s, [[0., 1.], [0., 1.]], 1., 1).unwrap();
        assert!(incomplete.image.is_none());
        assert_eq!(incomplete.spans, 1);
        let complete = bounds(&s, [[0., 1.], [0., 1.]], 1., 2).unwrap();
        assert!(complete.image.is_some());
        assert_eq!(complete.spans, 2);
        enclosed(complete.image.unwrap(), [0.5, 0.3, 1.]);
        for distance in [f64::NAN, f64::INFINITY] {
            assert!(bounds(&s, [[0., 1.], [0., 1.]], distance, 2).is_err());
        }
        assert!(bounds(&s, [[-0.1, 1.], [0., 1.]], 1., 2).is_err());
    }
    #[test]
    fn offset_intersection_excludes_only_complete_disjoint_carriers() {
        let a = plane();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                p[2] = 3.;
            }
        }
        let domains = [[[0., 1.], [0., 1.]]; 2];
        let r = intersection_candidates([&a, &b], domains, [1., -1.], 0.1, 100, 1).unwrap();
        assert_eq!(r.reason, "all-excluded");
        assert_eq!(r.visited_boxes, 1);
        assert_eq!(r.excluded_boxes, 1);
        assert!(r.boxes.is_empty() && r.pending.is_empty());
        for row in &mut b.control_points {
            for p in row {
                p[2] = 2.;
            }
        }
        let before = b.clone();
        // Original carriers are separated, but their offset carriers coincide.
        let r = intersection_candidates([&a, &b], domains, [1., -1.], 0.5, 100, 1).unwrap();
        assert_eq!(r.reason, "candidate-boxes");
        assert!(!r.boxes.is_empty());
        assert!(r.pending.is_empty());
        for t in [0., 0.23, 0.5, 0.77, 1.] {
            let uv = [[t, t], [t, t]];
            assert!(r.boxes.iter().any(|cell| (0..2).all(|side| {
                (0..2).all(|axis| {
                    cell[side][axis][0] <= uv[side][axis] && uv[side][axis] <= cell[side][axis][1]
                })
            })));
        }
        assert_eq!(b, before);
        let limited = intersection_candidates([&a, &b], domains, [1., -1.], 0.001, 1, 1).unwrap();
        assert_eq!(limited.visited_boxes, 1);
        assert_eq!(limited.reason, "work-limit");
        assert_eq!(limited.pending.len(), 2);
        assert_eq!(limited.excluded_boxes, 0);
    }
    #[test]
    fn singular_offset_regions_remain_pending_and_invalid_inputs_are_not_hidden() {
        let a = plane();
        let mut b = a.clone();
        b.control_points[1] = b.control_points[0].clone();
        let domains = [[[0., 1.], [0., 1.]]; 2];
        let r = intersection_candidates([&a, &b], domains, [1., -1.], 1., 1, 1).unwrap();
        assert_eq!(r.reason, "source-normal-unresolved");
        assert_eq!(r.pending.len(), 1);
        assert_eq!(r.excluded_boxes, 0);
        let mut invalid = b.clone();
        invalid.weights[0][0] = 0.;
        assert!(intersection_candidates([&a, &invalid], domains, [0., 1000.], 1., 1, 1).is_err());
        assert!(intersection_candidates([&a, &b], domains, [f64::NAN, 1.], 1., 1, 1).is_err());
    }
}
