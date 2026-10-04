//! Untrimmed multi-start helical thread profile patches; no solid/cap claim.
use crate::{Result, check, curve::Curve, helical_sweep, surface::Surface};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    External,
    Internal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Right,
    Left,
}
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub diameter: f64,
    pub pitch: f64,
    pub clearance: f64,
    pub starts: usize,
    pub turns: usize,
    pub kind: Kind,
    pub hand: Hand,
    pub error_budget: f64,
}
impl Default for Spec {
    fn default() -> Self {
        Self {
            diameter: 6.,
            pitch: 1.,
            clearance: 0.1,
            starts: 1,
            turns: 1,
            kind: Kind::External,
            hand: Hand::Right,
            error_budget: 1e-4,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Patch {
    pub surface: Surface,
    pub start: usize,
    pub turn: usize,
    pub quarter: usize,
    pub profile_segment: usize,
    pub real_arithmetic_error_estimate: f64,
}
#[derive(Clone, Debug)]
pub struct Patches {
    pub patches: Vec<Patch>,
    pub crest_radius: f64,
    pub root_radius: f64,
    pub lead: f64,
    /// Actual untrimmed coverage; differs from a bounded threaded solid's length.
    pub axial_extent: [f64; 2],
    pub rounding_certified: bool,
}
/// Sweep a sharp, truncated 60-degree profile with phase breakpoints
/// [0,1/16,3/8,5/8,15/16,1]. No cutter/root fillet or standard tolerance class.
/// Each integer turn is split into four quarter sweeps, five profile segments.
/// Adjacent starts are shifted by one pitch in Z. End planes are not trimmed.
pub fn patches(spec: Spec) -> Result<Patches> {
    check(
        [spec.diameter, spec.pitch, spec.clearance, spec.error_budget]
            .iter()
            .all(|v| v.is_finite()),
        "Thread dimensions must be finite",
    )?;
    check(
        spec.diameter > 0. && spec.pitch > 0. && spec.clearance >= 0. && spec.error_budget > 0.,
        "Thread diameter, pitch and error budget must be positive; clearance nonnegative",
    )?;
    check(
        (1..=8).contains(&spec.starts) && (1..=16).contains(&spec.turns),
        "Thread requires 1..8 starts and 1..16 turns",
    )?;
    let shift = if spec.kind == Kind::External {
        -spec.clearance / 2.
    } else {
        spec.clearance / 2.
    };
    let crest = spec.diameter / 2. + shift;
    let depth = 3_f64.sqrt() * spec.pitch * 5. / 16.;
    let root = crest - depth;
    check(
        root > 0. && root < crest && crest <= 1e9,
        "Thread root must remain positive and radius within coordinate limits",
    )?;
    let lead = spec.pitch * spec.starts as f64;
    let maximum_z = lead * (spec.turns as f64 + 1.);
    check(
        maximum_z.is_finite() && maximum_z <= 1e9,
        "Thread axial size exceeds coordinate limits",
    )?;
    let direction = if spec.hand == Hand::Right { 1. } else { -1. };
    let quarter_angle = direction * std::f64::consts::FRAC_PI_2;
    let phases = [0., 1. / 16., 3. / 8., 5. / 8., 15. / 16., 1.];
    let radii = [crest, crest, root, root, crest, crest];
    let mut out = Vec::with_capacity(20 * spec.starts * spec.turns);
    for start in 0..spec.starts {
        for turn in 0..spec.turns {
            for quarter in 0..4 {
                let z_offset =
                    start as f64 * spec.pitch + (turn as f64 + quarter as f64 / 4.) * lead;
                for segment in 0..5 {
                    let profile = Curve::from_polyline(vec![
                        vec![radii[segment], 0., z_offset + phases[segment] * spec.pitch],
                        vec![
                            radii[segment + 1],
                            0.,
                            z_offset + phases[segment + 1] * spec.pitch,
                        ],
                    ])?;
                    check(
                        profile.control_points[0][2] < profile.control_points[1][2],
                        "Thread pitch phase collapses at coordinate precision",
                    )?;
                    let a = helical_sweep::approximate(
                        &profile,
                        lead / 4.,
                        quarter as f64 * quarter_angle,
                        quarter_angle,
                        spec.error_budget,
                    )?;
                    out.push(Patch {
                        surface: a.surface,
                        start,
                        turn,
                        quarter,
                        profile_segment: segment,
                        real_arithmetic_error_estimate: a.real_arithmetic_error_estimate,
                    });
                }
            }
        }
    }
    Ok(Patches {
        patches: out,
        crest_radius: crest,
        root_radius: root,
        lead,
        axial_extent: [0., maximum_z],
        rounding_certified: false,
    })
}

/// Exact symbolic UV vertex of the binary64 affine clipping field.
/// Rectangle corners are [0,0],[1,0],[1,1],[0,1]; edge i connects i to (i+1)%4.
#[derive(Clone, Debug, PartialEq)]
pub enum ExactUvVertex {
    Corner { corner: usize },
    EdgePlane { edge: usize, z_limit: f64 },
}
#[derive(Clone, Debug)]
pub struct TrimCandidate {
    pub patch: Patch,
    /// Open list of polygon vertices, implicitly closed, in normalized UV.
    pub polygon_uv: Vec<[f64; 2]>,
    /// Exact convex polygon for z=gamma+alpha*u+beta*v, before rounding UV.
    pub exact_uv_vertices: Vec<ExactUvVertex>,
    pub affine_z_coefficients: [f64; 3],
    /// Outward bounds for Z on the entire retained polygon of the stored surface.
    pub retained_z_bounds: [f64; 2],
    /// Continuous error between stored Z and the authored affine trimming field.
    pub affine_z_error_upper_bound: f64,
    pub within_axial_tolerance: bool,
}
#[derive(Clone, Debug)]
pub struct ExcludedTrim {
    pub patch: Patch,
    /// Full original control-hull Z bounds prove disjointness from the slab.
    pub source_z_bounds: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct UnresolvedTrim {
    pub patch: Patch,
    /// Retains possible intersections when the candidate polygon disappeared.
    pub source_z_bounds: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct TrimCandidates {
    pub candidates: Vec<TrimCandidate>,
    pub excluded: Vec<ExcludedTrim>,
    pub unresolved: Vec<UnresolvedTrim>,
    pub requested_z_bounds: [f64; 2],
    /// Intersections are rounded candidates; no full trim topology claim.
    pub topology_certified: bool,
}
/// Candidate finite-end trimming, plus continuous outward Z evidence for each
/// retained polygon. No shell or complete intersection-coverage certificate.
pub fn trim_candidates(
    spec: Spec,
    z_bounds: [f64; 2],
    axial_tolerance: f64,
) -> Result<TrimCandidates> {
    use crate::distance_bounds::Interval as I;
    check(
        z_bounds.iter().all(|x| x.is_finite())
            && z_bounds[0] < z_bounds[1]
            && axial_tolerance.is_finite()
            && axial_tolerance > 0.,
        "Thread trim requires increasing finite Z limits and positive axial tolerance",
    )?;
    let source = patches(spec)?;
    let mut candidates = Vec::new();
    let mut excluded = Vec::new();
    let mut unresolved = Vec::new();
    for patch in source.patches {
        let s = &patch.surface;
        // Positive rational weights bound the complete stored surface by its
        // original control hull, independently of clipping or sampled vertices.
        let lo = s
            .control_points
            .iter()
            .flatten()
            .map(|p| p[2])
            .fold(f64::INFINITY, f64::min);
        let hi = s
            .control_points
            .iter()
            .flatten()
            .map(|p| p[2])
            .fold(f64::NEG_INFINITY, f64::max);
        let source_z_bounds = [lo, hi];
        if hi < z_bounds[0] || lo > z_bounds[1] {
            excluded.push(ExcludedTrim {
                patch,
                source_z_bounds,
            });
            continue;
        }

        let gamma = s.control_points[0][0][2];
        let alpha = s.control_points[1][0][2] - gamma;
        let beta = source.lead / 4.;
        let enclosure = |p: [f64; 2]| -> Result<I> {
            I::point(gamma)
                .add(I::point(alpha).mul(I::point(p[0]))?)?
                .add(I::point(beta).mul(I::point(p[1]))?)
        };
        let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let mut exact: Vec<_> = (0..4)
            .map(|corner| ExactUvVertex::Corner { corner })
            .collect();
        for (limit, lower) in [(z_bounds[0], true), (z_bounds[1], false)] {
            let sign = |vertex: &ExactUvVertex| {
                match vertex {
                    ExactUvVertex::Corner { corner } => {
                        let p = corners[*corner];
                        crate::exact_products::sum_sign(&[
                            (gamma, 1.),
                            (alpha, p[0]),
                            (beta, p[1]),
                            (-limit, 1.),
                        ])
                    }
                    // An exact first-plane intersection is classified against
                    // the parallel second plane by the original limits alone.
                    ExactUvVertex::EdgePlane { z_limit, .. } => z_limit.total_cmp(&limit),
                }
            };
            let inside = |sign: std::cmp::Ordering| {
                if lower {
                    sign != std::cmp::Ordering::Less
                } else {
                    sign != std::cmp::Ordering::Greater
                }
            };
            let mut clipped = Vec::new();
            for i in 0..exact.len() {
                let a = &exact[i];
                let b = &exact[(i + 1) % exact.len()];
                let sa = sign(a);
                let sb = sign(b);
                if inside(sa) {
                    clipped.push(a.clone());
                }
                if inside(sa) != inside(sb) {
                    let vertex = if sa == std::cmp::Ordering::Equal {
                        a.clone()
                    } else if sb == std::cmp::Ordering::Equal {
                        b.clone()
                    } else {
                        let incident = |v: &ExactUvVertex, e: usize| match v {
                            ExactUvVertex::Corner { corner } => {
                                *corner == e || *corner == (e + 1) % 4
                            }
                            ExactUvVertex::EdgePlane { edge, .. } => *edge == e,
                        };
                        let edge = (0..4)
                            .find(|&e| incident(a, e) && incident(b, e))
                            .ok_or_else(|| {
                                crate::Error::new(
                                    "NURBS_NUMERIC_ERROR",
                                    "Parallel thread planes produced a non-rectangle crossing",
                                )
                            })?;
                        ExactUvVertex::EdgePlane {
                            edge,
                            z_limit: limit,
                        }
                    };
                    clipped.push(vertex);
                }
            }
            clipped.dedup();
            if clipped.len() > 1 && clipped.first() == clipped.last() {
                clipped.pop();
            }
            exact = clipped;
        }
        if exact.len() < 3 {
            unresolved.push(UnresolvedTrim {
                patch,
                source_z_bounds,
            });
            continue;
        }
        let mut polygon = Vec::with_capacity(exact.len());
        for vertex in &exact {
            match vertex {
                ExactUvVertex::Corner { corner } => polygon.push(corners[*corner]),
                ExactUvVertex::EdgePlane { edge, z_limit } => {
                    // Compute only an enclosure/observation of the exact recipe.
                    let (fixed, coefficient, other) = match edge {
                        0 => ([0., 0.], alpha, 0.),
                        1 => ([1., 0.], beta, alpha),
                        2 => ([0., 1.], alpha, beta),
                        3 => ([0., 0.], beta, 0.),
                        _ => unreachable!("rectangle edge"),
                    };
                    let q = I::point(*z_limit)
                        .sub(I::point(gamma))?
                        .sub(I::point(other))?
                        .div(I::point(coefficient))?;
                    let lo = q.lo.max(0.);
                    let hi = q.hi.min(1.);
                    check(lo <= hi, "Exact thread trim vertex cannot be enclosed")?;
                    let t = lo * 0.5 + hi * 0.5;
                    polygon.push(if *edge == 0 || *edge == 2 {
                        [t, fixed[1]]
                    } else {
                        [fixed[0], t]
                    });
                }
            }
        }
        // Positive U/V bases partition unity. An affine field has the same
        // B-spline coefficients at Greville coordinates. Bounding every stored
        // coefficient difference therefore covers all UV, not just vertices.
        let mut error = 0_f64;
        for row in 0..2 {
            for j in 0..s.control_points[row].len() {
                let mut sum = I::point(0.);
                for &k in &s.knots_v[j + 1..=j + s.degree_v] {
                    sum = sum.add(I::point(k))?;
                }
                let greville = sum.div(I::point(s.degree_v as f64))?;
                let expected = I::point(gamma)
                    .add(I::point(alpha).mul(I::point(row as f64))?)?
                    .add(I::point(beta).mul(greville)?)?;
                let delta = I::point(s.control_points[row][j][2]).sub(expected)?;
                error = error.max(delta.lo.abs().max(delta.hi.abs()));
            }
        }
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &p in &polygon {
            let e = enclosure(p)?;
            lo = lo.min(e.lo);
            hi = hi.max(e.hi);
        }
        let z = I::new(lo, hi)?.add(I::new(-error, error)?)?;
        let allowed_lo = I::point(z_bounds[0]).sub(I::point(axial_tolerance))?;
        let allowed_hi = I::point(z_bounds[1]).add(I::point(axial_tolerance))?;
        let within = z.lo >= allowed_lo.hi && z.hi <= allowed_hi.lo;
        candidates.push(TrimCandidate {
            patch,
            polygon_uv: polygon,
            exact_uv_vertices: exact,
            affine_z_coefficients: [gamma, alpha, beta],
            retained_z_bounds: [z.lo, z.hi],
            affine_z_error_upper_bound: error,
            within_axial_tolerance: within,
        });
    }
    Ok(TrimCandidates {
        candidates,
        excluded,
        unresolved,
        requested_z_bounds: z_bounds,
        topology_certified: false,
    })
}

#[derive(Clone, Debug)]
pub struct PcurveApproximation {
    pub curve: Curve,
    /// Continuous Euclidean UV error against the exact linear-edge polygon,
    /// under matching segment parameters. Does not measure 3D surface error.
    pub max_uv_deviation_upper_bound: f64,
    pub vertex_enclosures: Vec<[[f64; 2]; 2]>,
}
impl TrimCandidate {
    /// Reconstruct from exact recipes and coefficients, without trusting the
    /// mutable polygon_uv observation. This certifies UV geometry only.
    pub fn pcurve(&self, max_uv_error: f64) -> Result<PcurveApproximation> {
        pcurve_from_exact_vertices(
            &self.exact_uv_vertices,
            self.affine_z_coefficients,
            max_uv_error,
        )
    }
}
/// Materialize a degree-one pcurve from exact affine clipping recipes. The
/// convex combination of endpoint errors bounds every point on every edge.
/// Does not certify polygon simplicity, patch linkage or B-rep topology.
pub fn pcurve_from_exact_vertices(
    vertices: &[ExactUvVertex],
    affine_z: [f64; 3],
    max_uv_error: f64,
) -> Result<PcurveApproximation> {
    use crate::distance_bounds::Interval as I;
    check(
        (3..=254).contains(&vertices.len())
            && affine_z.iter().all(|v| v.is_finite())
            && affine_z[1] > 0.
            && affine_z[2] > 0.
            && max_uv_error.is_finite()
            && max_uv_error > 0.,
        "Thread pcurve requires valid affine coefficients, 3..254 vertices and positive UV tolerance",
    )?;
    let [gamma, alpha, beta] = affine_z;
    let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut points = Vec::with_capacity(vertices.len() + 1);
    let mut enclosures = Vec::with_capacity(vertices.len());
    let mut error = 0_f64;
    for vertex in vertices {
        let bounds = match vertex {
            ExactUvVertex::Corner { corner } => {
                check(*corner < 4, "Invalid thread pcurve corner")?;
                corners[*corner].map(|x| [x, x])
            }
            ExactUvVertex::EdgePlane { edge, z_limit } => {
                check(
                    *edge < 4 && z_limit.is_finite(),
                    "Invalid thread pcurve edge-plane recipe",
                )?;
                let (coefficient, other) = match edge {
                    0 => (alpha, 0.),
                    1 => (beta, alpha),
                    2 => (alpha, beta),
                    3 => (beta, 0.),
                    _ => unreachable!(),
                };
                let q = I::point(*z_limit)
                    .sub(I::point(gamma))?
                    .sub(I::point(other))?
                    .div(I::point(coefficient))?;
                let range = [q.lo.max(0.), q.hi.min(1.)];
                check(
                    range[0] <= range[1],
                    "Thread pcurve recipe lies outside the rectangle edge",
                )?;
                match edge {
                    0 => [range, [0., 0.]],
                    1 => [[1., 1.], range],
                    2 => [range, [1., 1.]],
                    3 => [[0., 0.], range],
                    _ => unreachable!(),
                }
            }
        };
        let p = bounds.map(|b| b[0] * 0.5 + b[1] * 0.5);
        // Every recipe has at most one nonconstant UV coordinate; no sqrt
        // rounding or spurious underflow error is introduced at exact corners.
        for j in 0..2 {
            if bounds[j][0] != bounds[j][1] {
                error = error.max(
                    (p[j] - bounds[j][0])
                        .abs()
                        .max((p[j] - bounds[j][1]).abs())
                        .next_up(),
                );
            }
        }
        points.push(p.to_vec());
        enclosures.push(bounds);
    }
    check(
        error <= max_uv_error,
        "Thread pcurve cannot meet the requested UV error",
    )?;
    check(
        points.windows(2).all(|p| p[0] != p[1]) && points.first() != points.last(),
        "Thread pcurve vertices collapse at binary64 precision",
    )?;
    points.push(points[0].clone());
    let curve = Curve::from_polyline(points)?;
    Ok(PcurveApproximation {
        curve,
        max_uv_deviation_upper_bound: error,
        vertex_enclosures: enclosures,
    })
}

#[derive(Clone, Debug)]
pub struct MappedPcurveApproximation {
    pub pcurve: PcurveApproximation,
    pub derivative_norm_upper_bounds: [f64; 2],
    pub max_3d_deviation_upper_bound: f64,
    pub rectangles: usize,
}
impl TrimCandidate {
    /// Bound S(rounded_pcurve(t))-S(exact_UV_polygon(t)) continuously.
    /// This does not certify distance to a toroidal/axial plane or an edge
    /// fit in 3D; both paths are mapped through this same stored surface.
    pub fn pcurve_with_world_bound(
        &self,
        max_uv_error: f64,
        max_world_error: f64,
        max_rectangles: usize,
    ) -> Result<MappedPcurveApproximation> {
        use crate::distance_bounds::Interval as I;
        check(
            max_world_error.is_finite() && max_world_error > 0.,
            "Mapped pcurve requires positive finite world tolerance",
        )?;
        let surface = &self.patch.surface;
        surface.validate()?;
        check(
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ] == [0., 1., 0., 1.],
            "Thread pcurve mapping requires normalized UV surface storage",
        )?;
        let pcurve = self.pcurve(max_uv_error)?;
        let bounds = crate::surface_parameter_bounds::inspect(&self.patch.surface, max_rectangles)?;
        let mut uv_error = [0_f64; 2];
        for (p, b) in pcurve
            .curve
            .control_points
            .iter()
            .zip(&pcurve.vertex_enclosures)
        {
            for axis in 0..2 {
                if b[axis][0] != b[axis][1] {
                    uv_error[axis] = uv_error[axis].max(
                        (p[axis] - b[axis][0])
                            .abs()
                            .max((p[axis] - b[axis][1]).abs())
                            .next_up(),
                    );
                }
            }
        }
        let mut error = I::point(0.);
        for axis in 0..2 {
            if uv_error[axis] > 0. {
                error = error.add(
                    I::point(uv_error[axis])
                        .mul(I::point(bounds.derivative_norm_upper_bounds[axis]))?,
                )?;
            }
        }
        check(
            error.hi <= max_world_error,
            "Mapped pcurve cannot meet the requested world tolerance",
        )?;
        Ok(MappedPcurveApproximation {
            pcurve,
            derivative_norm_upper_bounds: bounds.derivative_norm_upper_bounds,
            max_3d_deviation_upper_bound: error.hi,
            rectangles: bounds.rectangles,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MappedEdge {
    pub curve: Curve,
    pub pcurve: Curve,
    pub agreement: crate::curve_surface_agreement::Report,
    pub requested_world_tolerance: f64,
}
#[derive(Clone, Debug)]
pub struct MappedEdges {
    pub edges: Vec<MappedEdge>,
    pub all_edges_within_tolerance: bool,
    /// Cumulative interval-verifier cells, including discarded refinement trials.
    pub verification_cells: usize,
}
impl TrimCandidate {
    /// Construct shared 3D edge candidates, accepting only full-interval
    /// C(t)=S(P(t)) verification. Sampling builds candidates, never proofs.
    /// The global cell budget is shared between all edges and refinement trials.
    pub fn mapped_edges(
        &self,
        max_uv_error: f64,
        world_tolerance: f64,
        max_spans: usize,
        max_verification_cells: usize,
    ) -> Result<MappedEdges> {
        use crate::curve_surface_agreement::{self, Report, Status};
        check(
            world_tolerance.is_finite()
                && world_tolerance > 0.
                && (1..=255).contains(&max_spans)
                && max_verification_cells <= 1_000_000,
            "Mapped edges require positive world tolerance, 1..255 spans and at most 1000000 cells",
        )?;
        self.patch.surface.validate()?;
        let pcurve = self.pcurve(max_uv_error)?;
        let mut edges = Vec::new();
        let mut used = 0;
        for pair in pcurve.curve.control_points.windows(2) {
            let edge_pcurve = Curve::from_polyline(vec![pair[0].clone(), pair[1].clone()])?;
            let mut natural = None;
            for fixed in 0..2 {
                let varying = 1-fixed;
                if pair[0][fixed] == pair[1][fixed] && [0.,1.].contains(&pair[0][fixed])
                    && pair[0][varying] != pair[1][varying] {
                    let axis = if fixed == 0 { crate::surface::Axis::U } else { crate::surface::Axis::V };
                    let mut c = self.patch.surface.iso(axis,pair[0][fixed])?;
                    let lo = pair[0][varying].min(pair[1][varying]);
                    let hi = pair[0][varying].max(pair[1][varying]);
                    if lo != 0. || hi != 1. {
                        c = c.trim(lo,hi)?;
                        let [a,b] = c.domain();
                        c.knots = c.knots.iter().map(|&t| (t-a)/(b-a)).collect();
                    }
                    natural = Some(if pair[0][varying] > pair[1][varying] { c.reverse()? } else { c });
                }
            }
            let composed = if natural.is_none() {
                crate::curve_surface_composition::trace_candidate(&edge_pcurve,&self.patch.surface)?
            } else { None };
            let mut spans = 1;
            loop {
                let mut points = Vec::with_capacity(spans + 1);
                for i in 0..=spans {
                    let t = i as f64 / spans as f64;
                    let uv = if i == 0 { pair[0].clone() } else if i == spans { pair[1].clone() }
                        else { edge_pcurve.evaluate(t)?.point };
                    let endpoint = |knots: &[f64], degree: usize, count: usize, t: f64| {
                        if t == knots[degree] && knots[..=degree].iter().all(|&k| k == t) { Some(0) }
                        else if t == knots[count] && knots[knots.len()-degree-1..].iter().all(|&k| k == t) { Some(count-1) }
                        else { None }
                    };
                    let s = &self.patch.surface;
                    let corner = (if s.periodic_u { None } else { endpoint(&s.knots_u,s.degree_u,s.control_points.len(),uv[0]) },
                        if s.periodic_v { None } else { endpoint(&s.knots_v,s.degree_v,s.control_points[0].len(),uv[1]) });
                    points.push(if let (Some(u),Some(v)) = corner {
                        s.control_points[u][v].clone()
                    } else { s.evaluate(uv[0],uv[1])?.point.to_vec() });
                }
                let curve = if let Some(c) = &natural {
                    let mut c = c.clone();
                    // Rounded knot insertion is qualified below. Use the same
                    // authored vertex values as the adjacent polygon edges.
                    let full = (pair[0][0] == pair[1][0] && [0.,1.].contains(&pair[0][1]) && [0.,1.].contains(&pair[1][1]))
                        || (pair[0][1] == pair[1][1] && [0.,1.].contains(&pair[0][0]) && [0.,1.].contains(&pair[1][0]));
                    if !full {
                        c.control_points[0] = points[0].clone();
                        *c.control_points.last_mut().unwrap() = points.last().unwrap().clone();
                    }
                    c.validate()?;
                    c
                } else if let Some(c) = &composed {
                    let mut c = c.clone();
                    c.control_points[0] = points[0].clone();
                    *c.control_points.last_mut().unwrap() = points.last().unwrap().clone();
                    c
                } else { Curve::from_polyline(points)? };
                let remaining = max_verification_cells - used;
                let agreement = if remaining == 0 {
                    Report {
                        status: Status::Unresolved,
                        cells: 0,
                        witness: None,
                        witness_distance: None,
                    }
                } else {
                    curve_surface_agreement::verify(
                        &curve,
                        &edge_pcurve,
                        &self.patch.surface,
                        false,
                        world_tolerance,
                        remaining.min(100_000),
                    )?
                };
                used += agreement.cells;
                if agreement.status == Status::WithinTolerance
                    || natural.is_some()
                    || composed.is_some()
                    || spans == max_spans
                    || used == max_verification_cells
                {
                    edges.push(MappedEdge {
                        curve,
                        pcurve: edge_pcurve,
                        agreement,
                        requested_world_tolerance: world_tolerance,
                    });
                    break;
                }
                spans = (spans * 2).min(max_spans);
            }
        }
        let all_edges_within_tolerance = edges
            .iter()
            .all(|e| e.agreement.status == Status::WithinTolerance);
        Ok(MappedEdges {
            edges,
            all_edges_within_tolerance,
            verification_cells: used,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PlanarEdge {
    pub polygon_edge: usize,
    pub mapped: MappedEdge,
}
#[derive(Clone, Debug)]
pub struct PlanarEdges {
    pub z_plane: f64,
    pub edges: Vec<PlanarEdge>,
    pub all_edges_within_tolerance: bool,
    pub verification_cells: usize,
    /// Fragments do not certify a closed cap loop or shared shell topology.
    pub topology_certified: bool,
}
impl TrimCandidate {
    /// Boundary indices lying exactly on a plane of the authored affine UV
    /// clipping field. This is symbolic incidence, not stored-surface agreement.
    pub fn plane_boundary_indices(&self, z_plane: f64) -> Result<Vec<usize>> {
        check(
            z_plane.is_finite()
                && self.affine_z_coefficients.iter().all(|v| v.is_finite())
                && (3..=254).contains(&self.exact_uv_vertices.len()),
            "Invalid plane-boundary recipes",
        )?;
        for v in &self.exact_uv_vertices {
            check(
                match v {
                    ExactUvVertex::Corner { corner } => *corner < 4,
                    ExactUvVertex::EdgePlane { edge, z_limit } => *edge < 4 && z_limit.is_finite(),
                },
                "Invalid exact UV vertex recipe",
            )?;
        }
        let [gamma, alpha, beta] = self.affine_z_coefficients;
        let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let on_plane = |v: &ExactUvVertex| match v {
            ExactUvVertex::EdgePlane { z_limit, .. } => *z_limit == z_plane,
            ExactUvVertex::Corner { corner } => {
                let p = corners[*corner];
                crate::exact_products::sum_sign(&[
                    (gamma, 1.),
                    (alpha, p[0]),
                    (beta, p[1]),
                    (-z_plane, 1.),
                ]) == std::cmp::Ordering::Equal
            }
        };
        Ok((0..self.exact_uv_vertices.len())
            .filter(|&i| {
                on_plane(&self.exact_uv_vertices[i])
                    && on_plane(&self.exact_uv_vertices[(i + 1) % self.exact_uv_vertices.len()])
            })
            .collect())
    }
    /// Return only exact affine-polygon edges on z_plane. Every 3D control Z
    /// is set to that original binary64 plane value, then the entire modified
    /// curve is reverified against its side-surface pcurve.
    pub fn planar_edges(
        &self,
        z_plane: f64,
        max_uv_error: f64,
        world_tolerance: f64,
        max_spans: usize,
        max_verification_cells: usize,
    ) -> Result<PlanarEdges> {
        use crate::curve_surface_agreement::{self, Report, Status};
        check(z_plane.is_finite(), "Thread cap plane must be finite")?;
        let mut mapped = self.mapped_edges(
            max_uv_error,
            world_tolerance,
            max_spans,
            max_verification_cells,
        )?;
        let indices = self.plane_boundary_indices(z_plane)?;
        let mut edges = Vec::new();
        for (index, mut edge) in mapped.edges.into_iter().enumerate() {
            if !indices.contains(&index) {
                continue;
            }
            for p in &mut edge.curve.control_points {
                p[2] = z_plane;
            }
            edge.curve.validate()?;
            let remaining = max_verification_cells - mapped.verification_cells;
            edge.agreement = if remaining == 0 {
                Report {
                    status: Status::Unresolved,
                    cells: 0,
                    witness: None,
                    witness_distance: None,
                }
            } else {
                curve_surface_agreement::verify(
                    &edge.curve,
                    &edge.pcurve,
                    &self.patch.surface,
                    false,
                    world_tolerance,
                    remaining.min(100_000),
                )?
            };
            mapped.verification_cells += edge.agreement.cells;
            edges.push(PlanarEdge {
                polygon_edge: index,
                mapped: edge,
            });
        }
        let all_edges_within_tolerance = !edges.is_empty()
            && edges
                .iter()
                .all(|e| e.mapped.agreement.status == Status::WithinTolerance);
        Ok(PlanarEdges {
            z_plane,
            edges,
            all_edges_within_tolerance,
            verification_cells: mapped.verification_cells,
            topology_certified: false,
        })
    }
}
