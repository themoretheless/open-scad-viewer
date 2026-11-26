//! Coincident faces and post-Boolean cleanup.
//!
//! * [`classify_carrier_coincidence`] decides, before any surface/surface
//!   marching, whether two faces lie on one carrier within the effective
//!   tolerance. Literal/equivalent NURBS carriers and exact plane/cylinder
//!   evidence come from `boolean_robustness::preclassify_coincident_faces`;
//!   analytic recognition extends this to planes, spheres, cylinders, cones
//!   and tori whose control nets differ.
//! * [`route_face_pairs`] partitions all box-overlapping face pairs of two
//!   operands into coincident pairs (2D domain work), near-coincident pairs
//!   (tolerance arbitration, never marched) and pairs that need SSI marching.
//! * [`coplanar_face_region_boolean`] computes the Boolean of the trimmed
//!   regions of two coplanar faces in one shared planar chart.
//! * [`merge_coplanar_faces`] merges edge-adjacent coplanar faces of a planar
//!   polygonal model, cancels their shared edges and removes vertices left
//!   collinear between exactly two edges, then rebuilds valid topology.
use super::*;
use crate::boolean_robustness::{CoincidentFaceClass, preclassify_coincident_faces, surface_uv_domains};
use crate::same_surface_uv_boolean::{
    SameSurfaceUvLimits, UvBooleanOperation, UvRegion, boolean_uv_regions,
};
use nurbs_core::analysis::analytic_recognition::{
    AnalyticPrimitive, Recognition, certify_primitive, recognize_analytic_surface,
};

mod exact_overlap;
pub use exact_overlap::{
    exact_coplanar_intersection_patch, exact_coplanar_intersection_patches,
    exact_convex_coplanar_overlap,
};

/// Carrier kind of a coincident face pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarrierKind {
    SameNurbs,
    Plane,
    Sphere,
    Cylinder,
    Cone,
    Torus,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarrierCoincidence {
    /// Both faces lie on one carrier within the effective tolerance.
    Coincident {
        kind: CarrierKind,
        opposite_normals: bool,
        /// Upper bound of the carrier mismatch used for the decision.
        deviation_mm: f64,
    },
    /// Inside the 4x arbitration band: must not be marched, must not be merged.
    NearCoincident { kind: CarrierKind, deviation_mm: f64 },
    Distinct,
}

impl CarrierCoincidence {
    pub fn is_coincident(&self) -> bool {
        matches!(self, Self::Coincident { .. })
    }
}

fn implicit(primitive: &AnalyticPrimitive, p: [f64; 3]) -> f64 {
    match *primitive {
        AnalyticPrimitive::Plane { normal, offset } => dot(normal, p) + offset,
        AnalyticPrimitive::Sphere { center, radius } => norm(sub(p, center)) - radius,
        AnalyticPrimitive::Cylinder { point, axis, radius } => {
            let d = sub(p, point);
            norm(sub(d, mul(axis, dot(d, axis)))) - radius
        }
        AnalyticPrimitive::Cone {
            apex,
            axis,
            half_angle,
        } => {
            let d = sub(p, apex);
            let axial = dot(d, axis);
            let radial = norm(sub(d, mul(axis, axial)));
            radial * half_angle.cos() - axial * half_angle.sin()
        }
        AnalyticPrimitive::Torus {
            center,
            axis,
            major_radius,
            minor_radius,
        } => {
            let d = sub(p, center);
            let z = dot(d, axis);
            let rho = norm(sub(d, mul(axis, z)));
            ((rho - major_radius).powi(2) + z * z).sqrt() - minor_radius
        }
    }
}

fn implicit_gradient(primitive: &AnalyticPrimitive, p: [f64; 3], h: f64) -> [f64; 3] {
    std::array::from_fn(|i| {
        let mut a = p;
        let mut b = p;
        a[i] += h;
        b[i] -= h;
        (implicit(primitive, a) - implicit(primitive, b)) / (2. * h)
    })
}

fn kind_of(primitive: &AnalyticPrimitive) -> CarrierKind {
    match primitive {
        AnalyticPrimitive::Plane { .. } => CarrierKind::Plane,
        AnalyticPrimitive::Sphere { .. } => CarrierKind::Sphere,
        AnalyticPrimitive::Cylinder { .. } => CarrierKind::Cylinder,
        AnalyticPrimitive::Cone { .. } => CarrierKind::Cone,
        AnalyticPrimitive::Torus { .. } => CarrierKind::Torus,
    }
}

fn parallel(a: [f64; 3], b: [f64; 3]) -> bool {
    dot(a, b).abs() >= 1. - 1e-9
}

/// Parameter mismatch between two recognized primitives in millimetres, or
/// `None` when they are of different kinds or not parallel.
fn primitive_mismatch(a: &AnalyticPrimitive, b: &AnalyticPrimitive, size: f64) -> Option<f64> {
    use AnalyticPrimitive::*;
    match (*a, *b) {
        (
            Plane { normal: n1, offset: o1 },
            Plane { normal: n2, offset: o2 },
        ) => parallel(n1, n2).then(|| (o1 - dot(n1, n2).signum() * o2).abs()),
        (
            Sphere { center: c1, radius: r1 },
            Sphere { center: c2, radius: r2 },
        ) => Some(norm(sub(c1, c2)) + (r1 - r2).abs()),
        (
            Cylinder { point: p1, axis: a1, radius: r1 },
            Cylinder { point: p2, axis: a2, radius: r2 },
        ) => parallel(a1, a2).then(|| {
            let d = sub(p2, p1);
            norm(sub(d, mul(a1, dot(d, a1)))) + (r1 - r2).abs()
        }),
        (
            Cone { apex: p1, axis: a1, half_angle: t1 },
            Cone { apex: p2, axis: a2, half_angle: t2 },
        ) => (dot(a1, a2) >= 1. - 1e-9)
            .then(|| norm(sub(p1, p2)) + (t1 - t2).abs() * size),
        (
            Torus { center: c1, axis: a1, major_radius: r1, minor_radius: s1 },
            Torus { center: c2, axis: a2, major_radius: r2, minor_radius: s2 },
        ) => parallel(a1, a2)
            .then(|| norm(sub(c1, c2)) + (r1 - r2).abs() + (s1 - s2).abs()),
        _ => None,
    }
}

fn surface_box(surface: &Surface) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in surface.control_points.iter().flatten() {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

fn sample_point_normal(surface: &Surface) -> Option<([f64; 3], [f64; 3])> {
    let d = surface_uv_domains(surface);
    let e = surface
        .evaluate((d[0][0] + d[0][1]) / 2., (d[1][0] + d[1][1]) / 2.)
        .ok()?;
    let (su, sv) = e.first_derivatives()?;
    Some((e.point, cross(su, sv)))
}

/// Cylinder candidate from sampled differential geometry: the axis is normal
/// to two surface normals, the radius is the circumradius of three section
/// points. Only proposals; acceptance is the coefficient certificate.
fn cylinder_candidate(surface: &Surface) -> Option<AnalyticPrimitive> {
    let d = surface_uv_domains(surface);
    let at = |s: f64, t: f64| -> Option<([f64; 3], [f64; 3])> {
        let e = surface
            .evaluate(d[0][0] + s * (d[0][1] - d[0][0]), d[1][0] + t * (d[1][1] - d[1][0]))
            .ok()?;
        let (su, sv) = e.first_derivatives()?;
        Some((e.point, cross(su, sv)))
    };
    let samples = [at(0.2, 0.5)?, at(0.5, 0.5)?, at(0.8, 0.5)?];
    let axis = cross(samples[0].1, samples[2].1);
    let axis = unit(axis).ok()?;
    let flat = |p: [f64; 3]| sub(p, mul(axis, dot(p, axis)));
    let (a, b, c) = (flat(samples[0].0), flat(samples[1].0), flat(samples[2].0));
    let (ab, ac) = (sub(b, a), sub(c, a));
    let n = cross(ab, ac);
    let nn = dot(n, n);
    if nn <= 1e-18 {
        return None;
    }
    let center = add(
        a,
        mul(
            add(mul(cross(n, ab), dot(ac, ac)), mul(cross(ac, n), dot(ab, ab))),
            0.5 / nn,
        ),
    );
    let radius = norm(sub(a, center));
    Some(AnalyticPrimitive::Cylinder { point: center, axis, radius })
}

fn recognize(surface: &Surface, tolerance: f64) -> Result<Option<Recognition>> {
    if let Some(r) = recognize_analytic_surface(surface, tolerance)? {
        return Ok(Some(r));
    }
    let Some(primitive) = cylinder_candidate(surface) else {
        return Ok(None);
    };
    Ok(certify_primitive(surface, &primitive)?
        .filter(|&bound| bound <= tolerance)
        .map(|distance_upper| Recognition {
            primitive,
            distance_upper,
        }))
}

/// Classify whether two faces share a carrier, before any marching.
pub fn classify_carrier_coincidence(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
) -> Result<CarrierCoincidence> {
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    match preclassify_coincident_faces(a, face_a, b, face_b)? {
        CoincidentFaceClass::IdenticalCarrier => {
            return Ok(CarrierCoincidence::Coincident {
                kind: CarrierKind::SameNurbs,
                opposite_normals: false,
                deviation_mm: 0.,
            });
        }
        CoincidentFaceClass::EquivalentNurbs(map) => {
            return Ok(CarrierCoincidence::Coincident {
                kind: CarrierKind::SameNurbs,
                opposite_normals: map.opposite_normals,
                deviation_mm: 0.,
            });
        }
        CoincidentFaceClass::CoincidentPlane { opposite_normals } => {
            return Ok(CarrierCoincidence::Coincident {
                kind: CarrierKind::Plane,
                opposite_normals,
                deviation_mm: 0.,
            });
        }
        CoincidentFaceClass::CoincidentCylinder {
            opposite_normals,
            max_deviation_mm,
        } => {
            return Ok(CarrierCoincidence::Coincident {
                kind: CarrierKind::Cylinder,
                opposite_normals,
                deviation_mm: max_deviation_mm,
            });
        }
        CoincidentFaceClass::NearCoincident | CoincidentFaceClass::NonCoincident => {}
    }
    let (sa, sb) = (&a.faces[face_a].surface, &b.faces[face_b].surface);
    let (Some(ra), Some(rb)) = (
        recognize(sa, tolerance)?,
        recognize(sb, tolerance)?,
    ) else {
        return Ok(CarrierCoincidence::Distinct);
    };
    let (la, ha) = surface_box(sa);
    let (lb, hb) = surface_box(sb);
    let size = norm(sub(ha, la)).max(norm(sub(hb, lb))).max(1.);
    let Some(mismatch) = primitive_mismatch(&ra.primitive, &rb.primitive, size) else {
        return Ok(CarrierCoincidence::Distinct);
    };
    let deviation = mismatch + ra.distance_upper + rb.distance_upper;
    let kind = kind_of(&ra.primitive);
    if deviation > tolerance * 4. {
        return Ok(CarrierCoincidence::Distinct);
    }
    if deviation > tolerance {
        return Ok(CarrierCoincidence::NearCoincident {
            kind,
            deviation_mm: deviation,
        });
    }
    let (Some((pa, na)), Some((pb, nb))) = (sample_point_normal(sa), sample_point_normal(sb)) else {
        return Ok(CarrierCoincidence::Distinct);
    };
    let h = (size * 1e-6).max(1e-9);
    // Material side: surface normal combined with the shell face sense.
    let side = |p, n, reversed: bool| (dot(implicit_gradient(&ra.primitive, p, h), n) >= 0.) != reversed;
    Ok(CarrierCoincidence::Coincident {
        kind,
        opposite_normals: side(pa, na, face_use_reversed(a, face_a))
            != side(pb, nb, face_use_reversed(b, face_b)),
        deviation_mm: deviation,
    })
}

/// Face pairs of two operands partitioned before SSI marching.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FacePairRouting {
    /// Same-carrier pairs, resolved by 2D CSG on the shared carrier.
    pub coincident: Vec<(usize, usize, CarrierCoincidence)>,
    /// Pairs inside the arbitration band. They are never marched.
    pub near_coincident: Vec<(usize, usize, CarrierCoincidence)>,
    /// Remaining box-overlapping pairs that need surface/surface marching.
    pub marching: Vec<(usize, usize)>,
}

/// Route every box-overlapping face pair of `a` and `b`.
pub fn route_face_pairs(a: &Model, b: &Model) -> Result<FacePairRouting> {
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let boxes = |m: &Model| -> Vec<_> { m.faces.iter().map(|f| surface_box(&f.surface)).collect() };
    let (ba, bb) = (boxes(a), boxes(b));
    let mut routing = FacePairRouting::default();
    for (i, (la, ha)) in ba.iter().enumerate() {
        for (j, (lb, hb)) in bb.iter().enumerate() {
            if (0..3).any(|k| la[k] > hb[k] + 4. * tolerance || lb[k] > ha[k] + 4. * tolerance) {
                continue;
            }
            match classify_carrier_coincidence(a, i, b, j)? {
                c @ CarrierCoincidence::Coincident { .. } => routing.coincident.push((i, j, c)),
                c @ CarrierCoincidence::NearCoincident { .. } => {
                    routing.near_coincident.push((i, j, c))
                }
                CarrierCoincidence::Distinct => routing.marching.push((i, j)),
            }
        }
    }
    Ok(routing)
}

/// A planar region in model space: outer loop counter-clockwise about
/// `normal`, holes clockwise.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarRegion {
    pub normal: [f64; 3],
    pub outer: Vec<[f64; 3]>,
    pub holes: Vec<Vec<[f64; 3]>>,
}

fn face_use_reversed(model: &Model, face: usize) -> bool {
    model
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .find(|u| u.face == face)
        .is_some_and(|u| u.reversed)
}

/// Oriented boundary rings of a face (vertex ids), outer first, in the
/// material-left orientation of its shell use.
fn oriented_rings(model: &Model, face: usize) -> Result<Vec<Vec<usize>>> {
    let reversed = face_use_reversed(model, face);
    let f = &model.faces[face];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .map(|l| {
            let mut ids = loop_vertices(model, l)?;
            if reversed {
                ids.reverse();
            }
            Ok(ids)
        })
        .collect()
}

fn newell(points: &[[f64; 3]]) -> [f64; 3] {
    (0..points.len()).fold([0.; 3], |s, i| {
        add(
            s,
            cross(points[i], points[(i + 1) % points.len()]),
        )
    })
}

fn linear_face(model: &Model, face: usize) -> bool {
    let f = &model.faces[face];
    std::iter::once(f.outer).chain(f.holes.iter().copied()).all(|l| {
        model.loops[l]
            .coedges
            .iter()
            .all(|c| model.edges[c.edge].curve.degree == 1)
    })
}

/// Boolean of the trimmed regions of two coplanar, polygonally bounded faces,
/// computed in the shared planar chart of `face_a`. Opposite-normal pairs are
/// mapped into the same chart first, so the result is expressed with
/// `face_a`'s orientation.
pub fn coplanar_face_region_boolean(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
    operation: &str,
) -> Result<Vec<PlanarRegion>> {
    let operation = UvBooleanOperation::parse(operation)?;
    let class = classify_carrier_coincidence(a, face_a, b, face_b)?;
    if !matches!(
        class,
        CarrierCoincidence::Coincident {
            kind: CarrierKind::Plane | CarrierKind::SameNurbs,
            ..
        }
    ) {
        return Err(unsupported("Faces are not coplanar within the effective tolerance"));
    }
    if !linear_face(a, face_a) || !linear_face(b, face_b) {
        return Err(unsupported("Coplanar region Boolean needs polygonal face boundaries"));
    }
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let rings = |m: &Model, f| -> Result<Vec<Vec<[f64; 3]>>> {
        Ok(oriented_rings(m, f)?
            .into_iter()
            .map(|r| r.into_iter().map(|v| m.vertices[v].point).collect())
            .collect())
    };
    let ra = rings(a, face_a)?;
    let rb = rings(b, face_b)?;
    let normal = unit(newell(&ra[0]))?;
    let (u, v) = plane_basis(normal)?;
    let origin = ra[0][0];
    let to2 = |p: [f64; 3]| {
        let d = sub(p, origin);
        [dot(d, u), dot(d, v)]
    };
    let area2 = |l: &[[f64; 2]]| {
        (0..l.len())
            .map(|i| {
                let (p, q) = (l[i], l[(i + 1) % l.len()]);
                p[0] * q[1] - p[1] * q[0]
            })
            .sum::<f64>()
            / 2.
    };
    let region = |rings: &[Vec<[f64; 3]>]| -> UvRegion {
        let mut loops: Vec<Vec<[f64; 2]>> =
            rings.iter().map(|r| r.iter().map(|&p| to2(p)).collect()).collect();
        for (k, l) in loops.iter_mut().enumerate() {
            let want_positive = k == 0;
            if (area2(l) > 0.) != want_positive {
                l.reverse();
            }
        }
        let outer = loops.remove(0);
        UvRegion { outer, holes: loops }
    };
    let limits = SameSurfaceUvLimits {
        tolerance_uv: tolerance,
        ..SameSurfaceUvLimits::default()
    };
    let result = boolean_uv_regions(&[region(&ra)], &[region(&rb)], operation, limits)?;
    let lift = |q: &[f64; 2]| add(origin, add(mul(u, q[0]), mul(v, q[1])));
    Ok(result
        .iter()
        .map(|r| PlanarRegion {
            normal,
            outer: r.outer.iter().map(lift).collect(),
            holes: r.holes.iter().map(|h| h.iter().map(lift).collect()).collect(),
        })
        .collect())
}

/// Result of the post-Boolean planar cleanup.
#[derive(Clone, Debug, PartialEq)]
pub struct CoplanarMergeReport {
    pub merged_face_groups: usize,
    pub removed_faces: usize,
    pub removed_edges: usize,
    pub removed_vertices: usize,
}

/// Merge edge-adjacent coplanar faces of an all-planar polygonal model,
/// remove their shared (redundant) edges and vertices left collinear between
/// two edges, and rebuild a valid model. Persistent identities are inherited
/// from the source. Models with curved geometry, groups that would merge into
/// several disjoint outer loops, or pinched boundaries are refused with
/// `BREP_UNSUPPORTED_OPERATION` and left to the caller unchanged.
pub fn merge_coplanar_faces(model: &Model) -> Result<(Model, CoplanarMergeReport)> {
    model.validate()?;
    let tolerance = model.tolerance_mm;
    if model.faces.is_empty() {
        return Err(unsupported("Empty model has no faces to merge"));
    }
    if model.edges.iter().any(|e| e.curve.degree != 1)
        || (0..model.faces.len()).any(|f| face_plane(model, f).is_err())
    {
        return Err(unsupported("Coplanar face merge needs a planar polygonal model"));
    }
    let mut rings: Vec<Vec<Vec<usize>>> = (0..model.faces.len())
        .map(|f| oriented_rings(model, f))
        .collect::<Result<_>>()?;
    let points = |ids: &[usize]| -> Vec<[f64; 3]> { ids.iter().map(|&v| model.vertices[v].point).collect() };
    let planes: Vec<([f64; 3], f64)> = rings
        .iter()
        .map(|r| {
            let pts = points(&r[0]);
            let n = unit(newell(&pts))?;
            Ok((n, dot(n, pts[0])))
        })
        .collect::<Result<_>>()?;
    // Union edge-adjacent faces on one oriented plane.
    let mut edge_faces = vec![Vec::<usize>::new(); model.edges.len()];
    for (f, face) in model.faces.iter().enumerate() {
        for &l in std::iter::once(&face.outer).chain(&face.holes) {
            for c in &model.loops[l].coedges {
                edge_faces[c.edge].push(f);
            }
        }
    }
    let mut parent: Vec<usize> = (0..model.faces.len()).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        p[x] = r;
        r
    }
    let coplanar = |i: usize, j: usize| {
        dot(planes[i].0, planes[j].0) > 1. - 1e-9
            && (planes[i].1 - planes[j].1).abs() <= tolerance * 8.
    };
    let mut redundant = BTreeSet::new();
    for (e, owners) in edge_faces.iter().enumerate() {
        if let [x, y] = owners[..] {
            if x != y && coplanar(x, y) {
                redundant.insert(e);
                let (rx, ry) = (find(&mut parent, x), find(&mut parent, y));
                parent[rx] = ry;
            }
        }
    }
    if redundant.is_empty() {
        return Err(unsupported("No adjacent coplanar faces to merge"));
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for f in 0..model.faces.len() {
        let r = find(&mut parent, f);
        groups.entry(r).or_default().push(f);
    }
    let mut new_rings: Vec<Vec<Vec<usize>>> = vec![];
    let mut merged_face_groups = 0;
    let edge_between = |a: usize, b: usize| {
        model
            .edges
            .iter()
            .position(|e| e.vertices == [a, b] || e.vertices == [b, a])
    };
    for faces in groups.values() {
        if faces.len() == 1 {
            new_rings.push(std::mem::take(&mut rings[faces[0]]));
            continue;
        }
        // Directed boundary edges that survive cancellation of shared edges.
        let mut next = BTreeMap::<usize, usize>::new();
        let mut pinched = false;
        for &f in faces {
            for ring in &rings[f] {
                for i in 0..ring.len() {
                    let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
                    if edge_between(p, q).is_some_and(|e| redundant.contains(&e)) {
                        continue;
                    }
                    pinched |= next.insert(p, q).is_some();
                }
            }
        }
        if pinched {
            // A vertex with two outgoing boundary edges would need a choice.
            for &f in faces {
                new_rings.push(std::mem::take(&mut rings[f]));
            }
            continue;
        }
        let mut chains = vec![];
        while let Some((&start, _)) = next.iter().next() {
            let mut chain = vec![start];
            let mut at = next.remove(&start).unwrap();
            while at != start {
                chain.push(at);
                at = next
                    .remove(&at)
                    .ok_or_else(|| unsupported("Merged coplanar boundary is not closed"))?;
            }
            chains.push(chain);
        }
        let normal = planes[faces[0]].0;
        let (outers, holes): (Vec<_>, Vec<_>) = chains
            .into_iter()
            .partition(|c| dot(newell(&points(c)), normal) > 0.);
        if outers.len() != 1 {
            for &f in faces {
                new_rings.push(std::mem::take(&mut rings[f]));
            }
            continue;
        }
        merged_face_groups += 1;
        let mut merged = outers;
        merged.extend(holes);
        new_rings.push(merged);
    }
    if merged_face_groups == 0 {
        return Err(unsupported("Coplanar groups need a choice the cleanup does not make"));
    }
    // Remove vertices that now lie between exactly two collinear neighbors.
    let mut removed_vertices = 0;
    loop {
        let mut neighbors = BTreeMap::<usize, BTreeSet<usize>>::new();
        for ring in new_rings.iter().flatten() {
            for i in 0..ring.len() {
                let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
                neighbors.entry(p).or_default().insert(q);
                neighbors.entry(q).or_default().insert(p);
            }
        }
        let removable = neighbors.iter().find_map(|(&v, n)| {
            let [a, b] = n.iter().copied().collect::<Vec<_>>()[..] else {
                return None;
            };
            let (pa, pb, pv) = (
                model.vertices[a].point,
                model.vertices[b].point,
                model.vertices[v].point,
            );
            let ab = sub(pb, pa);
            let len = norm(ab);
            (len > tolerance
                && norm(cross(ab, sub(pv, pa))) / len <= tolerance
                && dot(sub(pv, pa), ab) > 0.
                && dot(sub(pv, pb), ab) < 0.)
                .then_some(v)
        });
        let Some(v) = removable else { break };
        for ring in new_rings.iter_mut().flatten() {
            ring.retain(|&x| x != v);
            if ring.len() < 3 {
                return Err(unsupported("Collinear cleanup collapsed a loop"));
            }
        }
        removed_vertices += 1;
    }
    let boundaries = new_rings
        .iter()
        .map(|r| PlanarBoundary {
            outer: points(&r[0]),
            holes: r[1..].iter().map(|h| points(h)).collect(),
        })
        .collect();
    let mut result = model_from_trimmed_polygons(boundaries, tolerance)?;
    result.inherit_topology_ids(&[model]);
    result.validate()?;
    let report = CoplanarMergeReport {
        merged_face_groups,
        removed_faces: model.faces.len() - result.faces.len(),
        removed_edges: model.edges.len().saturating_sub(result.edges.len()),
        removed_vertices,
    };
    Ok((result, report))
}
