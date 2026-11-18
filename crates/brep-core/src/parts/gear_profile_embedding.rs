//! Modular original-profile embedding: exact local covers and far hull witnesses.
use super::*;
use num_rational::BigRational as Q;
use num_traits::Zero;
use nurbs_core::interval_eval::Interval as I;
#[derive(Clone, Copy, Debug)]
pub struct ProfileEmbeddingLimits {
    pub local_certificates: usize,
    pub far_pairs: usize,
    pub exact_signs: usize,
}
impl Default for ProfileEmbeddingLimits {
    fn default() -> Self {
        Self {
            local_certificates: 257,
            far_pairs: 32768,
            exact_signs: 20000000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ProfileEmbeddingCertificate {
    pub teeth: usize,
    pub curves_per_tooth: usize,
    pub tooth_wire: usize,
    pub tolerance: f64,
    /// Exact original two-group source fans, in cyclic group order.
    pub local: Vec<crate::profile_certificate::ProfileCertificate>,
    pub circle: Option<crate::profile_certificate::ProfileCertificate>,
    /// One origin-line ray witness per nonadjacent pair, in lexicographic order.
    pub far_rays: Vec<[f64; 2]>,
}
#[derive(Clone, Debug)]
pub struct VerifiedGearProfile {
    profile: Vec<Vec<Curve>>,
    winding: Vec<i32>,
    radial_ranges: Vec<[f64; 2]>,
    teeth: usize,
}
impl VerifiedGearProfile {
    pub fn profile(&self) -> &[Vec<Curve>] {
        &self.profile
    }
    pub fn winding(&self) -> &[i32] {
        &self.winding
    }
    pub fn radial_ranges(&self) -> &[[f64; 2]] {
        &self.radial_ranges
    }
    pub fn teeth(&self) -> usize {
        self.teeth
    }
}
fn refuse(s: &str) -> Error {
    Error::new("BREP_GEAR_PROFILE_EMBEDDING_UNPROVEN", s)
}
fn validate(
    wires: &[Vec<Curve>],
    n: usize,
    k: usize,
    tooth: usize,
    tolerance: f64,
    limits: ProfileEmbeddingLimits,
) -> Result<usize> {
    if !(3..=256).contains(&n)
        || ![4, 6].contains(&k)
        || wires.is_empty()
        || wires.len() > 2
        || tooth >= wires.len()
        || wires[tooth].len() != n * k
        || !tolerance.is_finite()
        || tolerance <= 0.
        || limits.local_certificates > 257
        || limits.far_pairs > 32768
        || limits.exact_signs == 0
        || limits.exact_signs > 20000000
    {
        return Err(refuse("Invalid bounded grouped profile or proof limits"));
    }
    let far = if n == 3 { 0 } else { n * (n - 1) / 2 - n };
    if limits.local_certificates < n + wires.len() - 1 || limits.far_pairs < far {
        return Err(refuse("Insufficient complete local/far proof budget"));
    }
    for (wi, wire) in wires.iter().enumerate() {
        if wire.is_empty() || (wi != tooth && wire.len() > 128) {
            return Err(refuse("Profile contains an empty wire"));
        }
        for (i, c) in wire.iter().enumerate() {
            if c.degree == 0
                || c.degree > 8
                || c.control_points.len() > 256
                || c.weights.len() > 256
                || c.knots.len() > 273
            {
                return Err(refuse(
                    "Original curve exceeds the bounded modular source layout",
                ));
            }
            c.validate()?;
            let [a, b] = c.domain();
            if c.control_points[0].len() != 2
                || c.periodic
                || c.degree > 8
                || c.knots[..=c.degree].iter().any(|&v| v != a)
                || c.knots[c.control_points.len()..].iter().any(|&v| v != b)
                || c.control_points.iter().flatten().any(|v| v.abs() > 1e6)
                || c.control_points.last().unwrap() != &wire[(i + 1) % wire.len()].control_points[0]
            {
                return Err(refuse(
                    "Original profile requires bounded clamped exact closed planar curves",
                ));
            }
        }
    }
    Ok(far)
}
fn oriented_source(wire: Vec<Curve>, negative: bool) -> Vec<Vec<Curve>> {
    if !negative {
        return vec![wire];
    }
    // Preserve every original source bit and direction. The reused material-left
    // checker admits a clockwise source as a hole of this auxiliary CCW box.
    // Its independent hull/nesting checks establish the box relation as well.
    let r = wire
        .iter()
        .flat_map(|c| c.control_points.iter().flatten())
        .map(|v| v.abs())
        .fold(0_f64, f64::max)
        * 2.
        + 1.;
    let rectangle = vec![
        line2([-r, -r], [r, -r]),
        line2([r, -r], [r, r]),
        line2([r, r], [-r, r]),
        line2([-r, r], [-r, -r]),
    ];
    vec![rectangle, wire]
}
fn fan(wire: &[Curve], n: usize, k: usize, i: usize, negative: bool) -> Vec<Vec<Curve>> {
    let mut curves: Vec<_> = (0..2)
        .flat_map(|offset| {
            let g = (i + offset) % n;
            wire[g * k..(g + 1) * k].iter().cloned()
        })
        .collect();
    let first = curves[0].control_points[0].clone();
    let last = curves
        .last()
        .unwrap()
        .control_points
        .last()
        .unwrap()
        .clone();
    curves.push(line2([last[0], last[1]], [0., 0.]));
    curves.push(line2([0., 0.], [first[0], first[1]]));
    oriented_source(curves, negative)
}
fn groups(wire: &[Curve], k: usize) -> Vec<Vec<[Q; 2]>> {
    wire.chunks(k)
        .map(|curves| {
            curves
                .iter()
                .flat_map(|c| c.control_points.iter())
                .map(|p| [Q::from_float(p[0]).unwrap(), Q::from_float(p[1]).unwrap()])
                .collect()
        })
        .collect()
}
fn charge(work: &mut usize, limits: ProfileEmbeddingLimits) -> Result<()> {
    if *work >= limits.exact_signs {
        return Err(refuse("Exact sign budget exhausted"));
    }
    *work += 1;
    Ok(())
}
fn separated(
    a: &[[Q; 2]],
    b: &[[Q; 2]],
    ray: [f64; 2],
    work: &mut usize,
    limits: ProfileEmbeddingLimits,
) -> Result<bool> {
    if !ray.iter().all(|x| x.is_finite()) || ray == [0.; 2] {
        return Ok(false);
    }
    let r = [
        Q::from_float(ray[0]).unwrap(),
        Q::from_float(ray[1]).unwrap(),
    ];
    let mut side = 0i8;
    for (group, opposite) in [(a, false), (b, true)] {
        for p in group {
            charge(work, limits)?;
            let cross = &r[0] * &p[1] - &r[1] * &p[0];
            if cross.is_zero() {
                return Ok(false);
            }
            let sign = if cross > Q::zero() { 1 } else { -1 };
            if side == 0 {
                side = sign;
            }
            if sign != if opposite { -side } else { side } {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn adjacent(n: usize, i: usize, j: usize) -> bool {
    j == i + 1 || (i == 0 && j == n - 1)
}
fn rays(wire: &[Curve], k: usize, i: usize, j: usize) -> Vec<[f64; 2]> {
    let endpoint = |g: usize, last: bool| {
        if last {
            wire[(g + 1) * k - 1].control_points.last().unwrap()
        } else {
            &wire[g * k].control_points[0]
        }
    };
    let mut out = vec![[1., 0.], [0., 1.]];
    for x in [false, true] {
        for y in [false, true] {
            let a = endpoint(i, x);
            let b = endpoint(j, y);
            out.push([a[0] + b[0], a[1] + b[1]]);
        }
    }
    out
}
/// Positive rational basis implies the original image is in its control hull.
/// A strict positive dot bound keeps the entire curve/chord homotopy off origin.
fn radial_hull(c: &Curve) -> Result<[f64; 2]> {
    let a = &c.control_points[0];
    let b = c.control_points.last().unwrap();
    let n = [a[0] + b[0], a[1] + b[1]];
    let length = I::point(n[0])
        .mul(I::point(n[0]))?
        .add(I::point(n[1]).mul(I::point(n[1]))?)?
        .hi
        .sqrt()
        .next_up();
    if !length.is_finite() || length <= 0. {
        return Err(refuse("No radial hull direction"));
    }
    let mut lower = f64::INFINITY;
    let mut upper = 0_f64;
    for p in &c.control_points {
        let dot = I::point(p[0])
            .mul(I::point(n[0]))?
            .add(I::point(p[1]).mul(I::point(n[1]))?)?;
        lower = lower.min(dot.lo);
        let squared = I::point(p[0])
            .mul(I::point(p[0]))?
            .add(I::point(p[1]).mul(I::point(p[1]))?)?;
        upper = upper.max(squared.hi.max(0.).sqrt().next_up());
    }
    if lower <= 0. {
        return Err(refuse("Original control hull may meet the winding anchor"));
    }
    Ok([I::point(lower).div(I::point(length))?.lo, upper])
}
fn winding(wire: &[Curve], work: &mut usize, limits: ProfileEmbeddingLimits) -> Result<i32> {
    let mut out = 0;
    for c in wire {
        let a = &c.control_points[0];
        let b = c.control_points.last().unwrap();
        if (a[1] <= 0. && b[1] > 0.) || (b[1] <= 0. && a[1] > 0.) {
            charge(work, limits)?;
            let cross = Q::from_float(a[0]).unwrap() * Q::from_float(b[1]).unwrap()
                - Q::from_float(a[1]).unwrap() * Q::from_float(b[0]).unwrap();
            if a[1] <= 0. && cross > Q::zero() {
                out += 1;
            }
            if b[1] <= 0. && cross < Q::zero() {
                out -= 1;
            }
        }
    }
    Ok(out)
}
/// Produce bounded witnesses, then independently recheck every source/pair and
/// return an immutable owned copy of the verified original profile. No solid
/// certificate is issued. Limits of each reused exact local checker remain.
pub fn certify_profile_embedding(
    spec: &GearSpec,
    limits: ProfileEmbeddingLimits,
) -> Result<(ProfileEmbeddingCertificate, VerifiedGearProfile)> {
    let (wires, _) = profile_with_report(spec)?;
    let tooth = usize::from(spec.internal);
    let k = wires[tooth].len() / spec.teeth;
    certify_grouped_profile_embedding(&wires, spec.teeth, k, tooth, 1e-7, limits)
}
/// Certify an original grouped four/six-component tooth profile independently
/// of its generator. Original source curves and all local/far pairs are checked.
pub fn certify_grouped_profile_embedding(
    wires: &[Vec<Curve>],
    teeth: usize,
    k: usize,
    tooth: usize,
    tolerance: f64,
    limits: ProfileEmbeddingLimits,
) -> Result<(ProfileEmbeddingCertificate, VerifiedGearProfile)> {
    validate(wires, teeth, k, tooth, tolerance, limits)?;
    let wire = &wires[tooth];
    let points = groups(wire, k);
    let mut work = 0;
    let mut far_rays = Vec::new();
    for i in 0..teeth {
        for j in i + 1..teeth {
            if !adjacent(teeth, i, j) {
                let mut found = None;
                for ray in rays(wire, k, i, j) {
                    if separated(&points[i], &points[j], ray, &mut work, limits)? {
                        found = Some(ray);
                        break;
                    }
                }
                far_rays.push(
                    found.ok_or_else(|| refuse("No strict original far-group hull separator"))?,
                );
            }
        }
    }
    let mut local = Vec::new();
    for i in 0..teeth {
        local.push(crate::profile_certificate::certify(
            &fan(wire, teeth, k, i, tooth == 1),
            tolerance,
        )?);
    }
    let circle = if wires.len() == 2 {
        Some(crate::profile_certificate::certify(
            &oriented_source(wires[1 - tooth].clone(), tooth == 0),
            tolerance,
        )?)
    } else {
        None
    };
    let c = ProfileEmbeddingCertificate {
        teeth: teeth,
        curves_per_tooth: k,
        tooth_wire: tooth,
        tolerance: tolerance,
        local,
        circle,
        far_rays,
    };
    let verified = verify_profile_embedding(wires, &c, limits)?;
    Ok((c, verified))
}
/// Source-bound independent checker. Every original curve pair is covered by
/// a local adjacent-group certificate or an exact far-hull separator. Winding
/// transfers through origin-free control hulls; radial gaps prove nesting.
pub fn verify_profile_embedding(
    wires: &[Vec<Curve>],
    c: &ProfileEmbeddingCertificate,
    limits: ProfileEmbeddingLimits,
) -> Result<VerifiedGearProfile> {
    let far = validate(
        wires,
        c.teeth,
        c.curves_per_tooth,
        c.tooth_wire,
        c.tolerance,
        limits,
    )?;
    if c.local.len() != c.teeth
        || c.far_rays.len() != far
        || c.circle.is_some() != (wires.len() == 2)
    {
        return Err(refuse("Incomplete or extra modular profile witnesses"));
    }
    let wire = &wires[c.tooth_wire];
    let points = groups(wire, c.curves_per_tooth);
    let mut work = 0;
    let mut pair = 0;
    for i in 0..c.teeth {
        for j in i + 1..c.teeth {
            if !adjacent(c.teeth, i, j) {
                if !separated(&points[i], &points[j], c.far_rays[pair], &mut work, limits)? {
                    return Err(refuse("False original far-group separation witness"));
                }
                pair += 1;
            }
        }
    }
    for i in 0..c.teeth {
        crate::profile_certificate::verify(
            &fan(wire, c.teeth, c.curves_per_tooth, i, c.tooth_wire == 1),
            c.tolerance,
            &c.local[i],
        )?;
    }
    if let Some(circle) = &c.circle {
        crate::profile_certificate::verify(
            &oriented_source(wires[1 - c.tooth_wire].clone(), c.tooth_wire == 0),
            c.tolerance,
            circle,
        )?;
    }
    let mut winding_values = Vec::new();
    let mut ranges = Vec::new();
    for (wi, wire) in wires.iter().enumerate() {
        let mut range = [f64::INFINITY, 0_f64];
        for curve in wire {
            let hull = radial_hull(curve)?;
            let bound = if wi == c.tooth_wire {
                hull
            } else {
                nurbs_core::radial_bounds::curve_radius_error(curve, [0.; 3], [1.; 2])?
                    .ok_or_else(|| refuse("Circle-wire original radial range unresolved"))?
                    .radius_range
            };
            range[0] = range[0].min(bound[0]);
            range[1] = range[1].max(bound[1]);
        }
        let value = winding(wire, &mut work, limits)?;
        if value != if wi == 0 { 1 } else { -1 } {
            return Err(refuse(
                "Original profile winding/orientation differs from material roles",
            ));
        }
        winding_values.push(value);
        ranges.push(range);
    }
    if wires.len() == 2 && ranges[1][1] >= ranges[0][0] {
        return Err(refuse(
            "Original inner/outer radial separation remains unresolved",
        ));
    }
    Ok(VerifiedGearProfile {
        profile: wires.to_vec(),
        winding: winding_values,
        radial_ranges: ranges,
        teeth: c.teeth,
    })
}

/// Native gear body candidate with independently verified owned base profile
/// and inclusive nominal profile/wall accuracy. Whole-solid admission is separate.
#[derive(Clone, Debug)]
pub struct ProfileVerifiedGear {
    model: Model,
    geometry: GearGeometry,
    profile: VerifiedGearProfile,
}
impl ProfileVerifiedGear {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn geometry(&self) -> &GearGeometry {
        &self.geometry
    }
    pub fn profile(&self) -> &VerifiedGearProfile {
        &self.profile
    }
    pub fn solid_geometry_certified(&self) -> bool {
        false
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
/// Construct all four native gear families with original-profile embedding and
/// continuous nominal accuracy. The base model edges must match the immutable
/// verified original profile exactly. Does not issue a global solid certificate.
pub fn gear_with_verified_profile(
    spec: &GearSpec,
    limits: ProfileEmbeddingLimits,
) -> Result<ProfileVerifiedGear> {
    let (_, profile) = certify_profile_embedding(spec, limits)?;
    let (model, geometry) = gear_with_report(spec)?;
    if geometry.profile_error_upper.is_none() || geometry.ideal_wall_error_upper.is_none() {
        return Err(refuse(
            "Nominal profile or wall accuracy remains unqualified",
        ));
    }
    let validation = model.validate()?;
    if validation.boundary_edge_count != 0
        || model.bodies.len() != 1
        || model.shells.len() != 1
        || !model.shells[0].closed
    {
        return Err(refuse(
            "Native gear requires one closed owned body candidate",
        ));
    }
    let base: Vec<_> = profile.profile.iter().flatten().collect();
    if model.edges.len() < base.len() {
        return Err(refuse("Model omits verified base edges"));
    }
    for (source, edge) in base.iter().zip(&model.edges) {
        let saved = &edge.curve;
        if saved.degree != source.degree
            || saved.knots != source.knots
            || saved.weights != source.weights
            || saved.periodic != source.periodic
            || saved.control_points.len() != source.control_points.len()
            || saved
                .control_points
                .iter()
                .zip(&source.control_points)
                .any(|(p, q)| p.len() != 3 || p[0] != q[0] || p[1] != q[1] || p[2] != 0.)
        {
            return Err(refuse(
                "Model base differs from the verified original profile",
            ));
        }
    }
    Ok(ProfileVerifiedGear {
        model,
        geometry,
        profile,
    })
}
