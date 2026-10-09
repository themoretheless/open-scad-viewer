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
        if periodic[axis] && domain[axis][0] == start && domain[axis][1] < end {
            let extra = branches
                .iter()
                .map(|d| {
                    let mut mapped = *d;
                    mapped[axis] = [end; 2];
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
        if periodic[axis] && (domain[axis][0] == start || domain[axis][1] == end) {
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
    check(fixed.is_finite(), "Choose a finite fixed offset parameter")?;
    certify_contact_box(
        surfaces,
        distances,
        fixed_axis,
        [fixed; 2],
        first_other,
        second,
        max_spans,
    )
}

#[derive(Debug)]
pub enum ContactBand {
    Excluded,
    Unresolved,
    /// For every parameter in fixed_interval there is exactly one contact
    /// inside the supplied three-dimensional tube. Its dependence on that
    /// parameter is continuous. This says nothing about roots outside the tube.
    ContinuousBranch(crate::surface_contact::Witness),
}
/// Uniform contraction and inclusion over a complete parameter interval.
/// The residual at the tube midpoint retains the entire driving interval;
/// no sampled section is substituted for interval coverage. Source continuity
/// and a uniform contraction give continuous dependence of the unique root.
/// UVs are source parameters and point is an offset-center enclosure. Trim,
/// global branch completeness, envelope regularity and topology are unproved.
pub fn certify_contact_band(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed_interval: [f64; 2],
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<ContactBand> {
    check(
        fixed_interval.iter().all(|v| v.is_finite()) && fixed_interval[0] < fixed_interval[1],
        "Offset contact band needs a positive finite driving interval",
    )?;
    use crate::surface_contact::Verdict;
    Ok(
        match certify_contact_box(
            surfaces,
            distances,
            fixed_axis,
            fixed_interval,
            first_other,
            second,
            max_spans,
        )? {
            Verdict::Excluded => ContactBand::Excluded,
            Verdict::Unresolved => ContactBand::Unresolved,
            Verdict::Witness(w) => ContactBand::ContinuousBranch(w),
        },
    )
}

fn certify_contact_box(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed_interval: [f64; 2],
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<crate::surface_contact::Verdict> {
    use crate::surface_contact::{
        section_krawczyk_parameterized, SectionVerdict, Verdict, Witness,
    };
    check(
        fixed_axis < 2 && fixed_interval.iter().all(|v| v.is_finite()),
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
    first[fixed_axis] = fixed_interval;
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
    let driving_midpoint = fixed_interval[0] * 0.5 + fixed_interval[1] * 0.5;
    first_center[fixed_axis] = [driving_midpoint; 2];
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
    let driving = if fixed_interval[0] < fixed_interval[1] {
        Some((
            std::array::from_fn(|k| I {
                lo: a[fixed_axis][k][0],
                hi: a[fixed_axis][k][1],
            }),
            I::new(fixed_interval[0], fixed_interval[1])?.sub(I::point(driving_midpoint))?,
        ))
    } else {
        None
    };
    let (parameters, contraction_upper) =
        match section_krawczyk_parameterized(domain, jac, residual, driving)? {
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
#[path = "tests/source.rs"]
mod tests;
