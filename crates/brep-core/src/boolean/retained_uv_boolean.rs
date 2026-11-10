//! Retained 3D B-rep construction for certified linear UV arrangements.
//!
//! UV segments are lifted by exact rational Bezier composition on every
//! carrier knot cell. No sampled polyline or chord is substituted for a curved
//! boundary. Construction is staged and becomes visible only after structural
//! and full-interval boundary validation succeeds.
use crate::boolean_robustness::{classify_boolean_operand, surface_uv_domains, BooleanOperandKind};
use crate::same_surface_uv_boolean::{
    boolean_same_surface_uv_with_limits,
    SameSurfaceUvBooleanCertificate, SameSurfaceUvLimits, UvBooleanOperation, UvRegion,
};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopologyIds, TopologyLineageRecord, Vertex};
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct RetainedUvBooleanCertificate {
    pub version: u32,
    pub source_faces: [TopoId; 2],
    pub result_faces: Vec<TopoId>,
    pub same_sense: bool,
    pub uv: SameSurfaceUvBooleanCertificate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetainedUvBoolean {
    model: Model,
    certificate: RetainedUvBooleanCertificate,
}

impl RetainedUvBoolean {
    pub fn model(&self) -> &Model { &self.model }
    pub fn certificate(&self) -> &RetainedUvBooleanCertificate { &self.certificate }
    pub fn into_model(self) -> Model { self.model }
}

fn refused(message: impl Into<String>) -> Error {
    Error::new("BREP_RETAINED_UV_UNPROVEN", message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeKey {
    vertices: [(i64, i64); 2],
    a: (i64, i64),
    b: (i64, i64),
}

fn wrapped(surface: &Surface, mut uv: [f64; 2]) -> [f64; 2] {
    let domains = surface_uv_domains(surface);
    for axis in 0..2 {
        if [surface.periodic_u, surface.periodic_v][axis] {
            let period = domains[axis][1] - domains[axis][0];
            uv[axis] = domains[axis][0] + (uv[axis] - domains[axis][0]).rem_euclid(period);
        }
    }
    uv
}

fn key(uv: [f64; 2], tolerance: f64) -> Result<(i64, i64)> {
    if !tolerance.is_finite() || tolerance <= 0. || uv.iter().any(|v| !v.is_finite()) {
        return Err(refused("Retained UV key needs finite coordinates and tolerance"));
    }
    let mut out = [0_i64; 2];
    for axis in 0..2 {
        let value = uv[axis] / tolerance;
        if !value.is_finite() || value.abs() > i64::MAX as f64 {
            return Err(Error::new("BREP_RESOURCE_LIMIT", "Retained UV key range exhausted"));
        }
        out[axis] = value.round() as i64;
    }
    Ok((out[0], out[1]))
}

fn binomial(n: usize, k: usize) -> f64 {
    (1..=k.min(n - k)).fold(1., |value, i| value * (n - k + i) as f64 / i as f64)
}

/// Compose a straight global-UV segment with one clamped rational Bezier patch.
fn compose_segment(patch: &Surface, a: [f64; 2], b: [f64; 2]) -> Result<Curve> {
    let domain = surface_uv_domains(patch);
    let [du, dv] = [patch.degree_u, patch.degree_v];
    let degree = du.checked_add(dv).ok_or_else(|| Error::new("BREP_RESOURCE_LIMIT", "Composition degree overflow"))?;
    if degree > 64 {
        return Err(Error::new("BREP_RESOURCE_LIMIT", "Retained UV composition exceeds degree 64"));
    }
    let delta = [b[0] - a[0], b[1] - a[1]];
    let normalize = |axis: usize, value: f64| (value - domain[axis][0]) / (domain[axis][1] - domain[axis][0]);
    let endpoint = |axis: usize, t: f64| normalize(axis, a[axis] + delta[axis] * t);
    // Monomial coefficients of alpha^i(1-alpha)^(n-i), where alpha is linear
    // in the common normalized curve parameter.
    let factor_polynomial = |n: usize, i: usize, lo: f64, hi: f64| -> Vec<f64> {
        let mut alpha = vec![0.; n + 1];
        let mut beta = vec![0.; n + 1];
        for p in 0..=i {
            alpha[p] = binomial(i, p) * lo.powi((i - p) as i32) * (hi - lo).powi(p as i32);
        }
        for q in 0..=(n - i) {
            let sign = if q % 2 == 0 { 1. } else { -1. };
            beta[q] = binomial(n - i, q) * sign * (1. - lo).powi((n - i - q) as i32) * (hi - lo).powi(q as i32);
        }
        let mut out = vec![0.; n + 1];
        for p in 0..=i {
            for q in 0..=(n - i) { out[p + q] += alpha[p] * beta[q]; }
        }
        out
    };
    let u = [endpoint(0, 0.), endpoint(0, 1.)];
    let v = [endpoint(1, 0.), endpoint(1, 1.)];
    let mut numerator = vec![[0.; 4]; degree + 1];
    for i in 0..=du {
        for j in 0..=dv {
            let pu = factor_polynomial(du, i, u[0], u[1]);
            let pv = factor_polynomial(dv, j, v[0], v[1]);
            let mut product = vec![0.; degree + 1];
            for p in 0..=du {
                for q in 0..=dv { product[p + q] += pu[p] * pv[q]; }
            }
            // x^p = sum_(k=p..n) C(k,p)/C(n,p) B_k^n(x).
            for (power, coefficient) in product.into_iter().enumerate() {
                for k in power..=degree {
                    let conversion = binomial(k, power) / binomial(degree, power);
                    let w = patch.weights[i][j];
                    for axis in 0..3 {
                        numerator[k][axis] += coefficient * conversion * w * patch.control_points[i][j][axis];
                    }
                    numerator[k][3] += coefficient * conversion * w;
                }
            }
        }
    }
    if numerator.iter().flatten().any(|v| !v.is_finite()) {
        return Err(refused("Non-finite rational UV composition"));
    }
    let weights = numerator.iter().map(|p| p[3]).collect::<Vec<_>>();
    if weights.iter().any(|w| *w <= 0.) {
        return Err(refused("Rational UV composition has a non-positive pole weight"));
    }
    let control_points = numerator.iter().zip(&weights).map(|(p, w)| {
        vec![p[0] / w, p[1] / w, p[2] / w]
    }).collect();
    let mut knots = vec![0.; degree + 1];
    knots.extend(vec![1.; degree + 1]);
    let curve = Curve { degree, knots, control_points, weights, periodic: false };
    curve.validate()?;
    Ok(curve)
}

struct Piece {
    a: [f64; 2],
    b: [f64; 2],
    curve: Curve,
}

fn segment_pieces(surface: &Surface, patches: &[Surface], a: [f64; 2], b: [f64; 2], tolerance: f64) -> Result<Vec<Piece>> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let mut cuts = vec![0., 1.];
    for axis in 0..2 {
        if d[axis].abs() <= tolerance { continue; }
        let knots = if axis == 0 { &surface.knots_u } else { &surface.knots_v };
        for &knot in knots {
            let t = (knot - a[axis]) / d[axis];
            if t > tolerance && t < 1. - tolerance { cuts.push(t); }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() <= tolerance);
    let mut out = Vec::new();
    for pair in cuts.windows(2) {
        let point = |t: f64| [a[0] + d[0] * t, a[1] + d[1] * t];
        let raw_a = point(pair[0]);
        let raw_b = point(pair[1]);
        let mut p = raw_a;
        let mut q = raw_b;
        let mid = [(p[0] + q[0]) / 2., (p[1] + q[1]) / 2.];
        let wrapped_mid = wrapped(surface, mid);
        for axis in 0..2 {
            let shift = wrapped_mid[axis] - mid[axis];
            p[axis] += shift;
            q[axis] += shift;
        }
        let patch = patches.iter().find(|patch| {
            let domain = surface_uv_domains(patch);
            (0..2).all(|axis| {
                wrapped_mid[axis] >= domain[axis][0] - tolerance
                    && wrapped_mid[axis] <= domain[axis][1] + tolerance
            })
        }).ok_or_else(|| refused("UV segment midpoint has no carrier Bezier cell"))?;
        out.push(Piece { a: raw_a, b: raw_b, curve: compose_segment(patch, p, q)? });
    }
    Ok(out)
}

fn validate_staged(model: &Model) -> Result<()> {
    model.validate()?;
    model.validate_continuous(1_000_000)?;
    Ok(())
}

/// Build one open sheet from disjoint certified UV regions. Shared UV edges
/// are retained once and referenced by opposite coedge uses.
pub fn build_uv_region_sheet(
    surface: &Surface,
    regions: &[UvRegion],
    reversed: bool,
    tolerance_mm: f64,
    limits: SameSurfaceUvLimits,
) -> Result<Model> {
    surface.validate()?;
    if regions.is_empty() { return Model::empty(tolerance_mm); }
    let patches = nurbs_core::surface_edit::decompose(surface)?;
    let mut model = Model(
        brep_topology::Model {
            vertices: vec![], edges: vec![], loops: vec![], faces: vec![],
            shells: vec![Shell { faces: vec![], closed: false }], bodies: vec![], tolerance_mm,
        },
        TopologyIds::default(),
    );
    let mut vertices = BTreeMap::<(i64, i64), usize>::new();
    let mut edges = BTreeMap::<EdgeKey, usize>::new();
    let mut segment_count = 0_usize;
    for region in regions {
        let mut wires = Vec::new();
        for points in std::iter::once(&region.outer).chain(&region.holes) {
            if points.len() < 3 { return Err(refused("Retained UV loop has fewer than three vertices")); }
            let mut coedges = Vec::new();
            for i in 0..points.len() {
                for piece in segment_pieces(surface, &patches, points[i], points[(i + 1) % points.len()], limits.tolerance_uv)? {
                    segment_count += 1;
                    if segment_count > limits.max_segments {
                        return Err(Error::new("BREP_RESOURCE_LIMIT", "Retained UV edge budget exhausted"));
                    }
                    let wrapped_a = wrapped(surface, piece.a);
                    let wrapped_b = wrapped(surface, piece.b);
                    let vertex = |uv: [f64; 2], model: &mut Model, vertices: &mut BTreeMap<_, _>| -> Result<usize> {
                        let k = key(uv, limits.tolerance_uv)?;
                        if let Some(&id) = vertices.get(&k) { return Ok(id); }
                        let point = surface.evaluate(uv[0], uv[1])?.point;
                        let point = <[f64; 3]>::try_from(point).map_err(|_| refused("Surface point is not three-dimensional"))?;
                        let id = model.vertices.len();
                        model.vertices.push(Vertex { point });
                        vertices.insert(k, id);
                        Ok(id)
                    };
                    let va = vertex(wrapped_a, &mut model, &mut vertices)?;
                    let vb = vertex(wrapped_b, &mut model, &mut vertices)?;
                    if va == vb { return Err(refused("Retained UV segment collapses to one vertex")); }
                    let ka = key(wrapped_a, limits.tolerance_uv)?;
                    let kb = key(wrapped_b, limits.tolerance_uv)?;
                    let ra = key(piece.a, limits.tolerance_uv)?;
                    let rb = key(piece.b, limits.tolerance_uv)?;
                    let forward = (ka, ra) <= (kb, rb);
                    let edge_key = if forward {
                        EdgeKey { vertices: [ka, kb], a: ra, b: rb }
                    } else {
                        EdgeKey { vertices: [kb, ka], a: rb, b: ra }
                    };
                    let edge = if let Some(&edge) = edges.get(&edge_key) {
                        edge
                    } else {
                        let mut curve = piece.curve;
                        if !forward { curve = curve.reverse()?; }
                        let id = model.edges.len();
                        model.edges.push(Edge { degenerate: false, vertices: if forward { [va, vb] } else { [vb, va] }, curve });
                        edges.insert(edge_key, id);
                        id
                    };
                    coedges.push(Coedge::new(edge, !forward, crate::line(piece.a.to_vec(), piece.b.to_vec())));
                }
            }
            wires.push(model.loops.len());
            model.loops.push(Loop { coedges });
        }
        let face = model.faces.len();
        model.faces.push(Face { surface: surface.clone(), outer: wires[0], holes: wires[1..].to_vec() });
        model.shells[0].faces.push(FaceUse { face, reversed });
    }
    split_shell_components(&mut model);
    model.rebuild_topology_ids();
    validate_staged(&model)?;
    Ok(model)
}

/// Regions that share no edge are separate sheets: like UV islands of a cap,
/// each connected group of faces owns its own open shell, since a shell must
/// be connected. Groups keep first-face order.
fn split_shell_components(model: &mut Model) {
    let mut parent: Vec<usize> = (0..model.faces.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut owner = vec![usize::MAX; model.edges.len()];
    for (face, f) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&f.outer).chain(&f.holes) {
            for coedge in &model.loops[wire].coedges {
                if owner[coedge.edge] == usize::MAX {
                    owner[coedge.edge] = face;
                } else {
                    let (x, y) = (root(&mut parent, owner[coedge.edge]), root(&mut parent, face));
                    parent[x] = y;
                }
            }
        }
    }
    let uses = std::mem::take(&mut model.shells[0].faces);
    let mut groups: Vec<(usize, Vec<FaceUse>)> = Vec::new();
    for face_use in uses {
        let r = root(&mut parent, face_use.face);
        match groups.iter_mut().find(|(g, _)| *g == r) {
            Some((_, list)) => list.push(face_use),
            None => groups.push((r, vec![face_use])),
        }
    }
    if !groups.is_empty() {
        model.shells = groups.into_iter().map(|(_, faces)| Shell { faces, closed: false }).collect();
    }
}

fn sole_face_use(model: &Model, face: usize) -> Result<FaceUse> {
    let uses = model.shells.iter().flat_map(|s| &s.faces).filter(|u| u.face == face).collect::<Vec<_>>();
    match uses.as_slice() {
        [u] => Ok((*u).clone()),
        _ => Err(refused("Selected face must have exactly one shell use")),
    }
}

/// Execute a certified same-carrier operation on two single-face open sheets.
/// The result remains an open sheet; no body or closed shell is authored.
pub fn boolean_same_surface_uv_retained_with_limits(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
    equivalence: Option<crate::boolean_robustness::UvEquivalence>,
    operation: UvBooleanOperation,
    limits: SameSurfaceUvLimits,
) -> Result<RetainedUvBoolean> {
    a.validate()?;
    b.validate()?;
    if classify_boolean_operand(a)? != BooleanOperandKind::Sheet
        || classify_boolean_operand(b)? != BooleanOperandKind::Sheet
        || a.faces.len() != 1 || b.faces.len() != 1 || face_a != 0 || face_b != 0
    {
        return Err(refused("Retained face-pair Boolean currently admits two single-face open sheets"));
    }
    let ua = sole_face_use(a, face_a)?;
    let ub = sole_face_use(b, face_b)?;
    if ua.reversed != ub.reversed || equivalence.is_some_and(|map| map.opposite_normals) {
        return Err(refused("Sheet Boolean needs a common certified material side"));
    }
    let uv = boolean_same_surface_uv_with_limits(
        &a.faces[face_a].surface, a, face_a, &b.faces[face_b].surface, b, face_b,
        equivalence, operation, limits,
    )?;
    let mut model = build_uv_region_sheet(&a.faces[face_a].surface, uv.regions(), ua.reversed, a.tolerance_mm.max(b.tolerance_mm), limits)?;
    model.inherit_topology_ids(&[a, b]);
    let result_faces = model.1.faces.clone();
    if !result_faces.is_empty() {
        // Each result face merges both source faces; a merge has exactly one
        // child, so a result of several faces records one merge per face.
        for &child in &result_faces {
            model.1.lineage.push(TopologyLineageRecord {
                operation: "merge".into(), entity_kind: "face".into(),
                parents: vec![a.1.faces[face_a], b.1.faces[face_b]], children: vec![child],
            });
        }
        model.refresh_change_set(&[a, b]);
    }
    validate_staged(&model)?;
    let certificate = RetainedUvBooleanCertificate {
        version: 1, source_faces: [a.1.faces[face_a], b.1.faces[face_b]],
        result_faces, same_sense: ua.reversed, uv: uv.certificate().clone(),
    };
    Ok(RetainedUvBoolean { model, certificate })
}

pub fn boolean_same_surface_uv_retained(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
    equivalence: Option<crate::boolean_robustness::UvEquivalence>,
    operation: UvBooleanOperation,
) -> Result<RetainedUvBoolean> {
    boolean_same_surface_uv_retained_with_limits(a, face_a, b, face_b, equivalence, operation, SameSurfaceUvLimits::default())
}

pub fn verify_same_surface_uv_retained(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
    result: &RetainedUvBoolean,
) -> Result<()> {
    if result.certificate.version != 1 { return Err(refused("Retained UV certificate version mismatch")); }
    let fresh = boolean_same_surface_uv_retained_with_limits(
        a, face_a, b, face_b, result.certificate.uv.equivalence,
        result.certificate.uv.operation, result.certificate.uv.limits,
    )?;
    if fresh != *result { return Err(refused("Retained UV replay differs from its certificate")); }
    Ok(())
}

fn replace_faces(source: &Model, selected: &[usize], shell: usize, reversed: bool, sheet: Model) -> Result<Model> {
    let chosen = selected.iter().copied().collect::<BTreeSet<_>>();
    let mut model = source.clone();
    model.faces.clear();
    model.loops.clear();
    let mut retained_faces = vec![None; source.faces.len()];
    for (i, face) in source.faces.iter().enumerate() {
        if chosen.contains(&i) { continue; }
        let mut face = face.clone();
        face.outer = model.loops.len();
        model.loops.push(source.loops[face.outer].clone());
        face.holes.clear();
        for &wire in &source.faces[i].holes {
            face.holes.push(model.loops.len());
            model.loops.push(source.loops[wire].clone());
        }
        retained_faces[i] = Some(model.faces.len());
        model.faces.push(face);
    }
    let edge_offset = model.edges.len();
    let vertex_offset = model.vertices.len();
    model.edges.extend(sheet.edges.iter().cloned().map(|mut edge| {
        edge.vertices.iter_mut().for_each(|v| *v += vertex_offset);
        edge
    }));
    model.vertices.extend(sheet.vertices.iter().cloned());
    let loop_offset = model.loops.len();
    model.loops.extend(sheet.loops.iter().cloned().map(|mut wire| {
        wire.coedges.iter_mut().for_each(|c| c.edge += edge_offset);
        wire
    }));
    let new_face = model.faces.len();
    for face in &sheet.faces {
        let mut face = face.clone();
        face.outer += loop_offset;
        face.holes.iter_mut().for_each(|wire| *wire += loop_offset);
        model.faces.push(face);
    }
    for (index, usage) in model.shells.iter_mut().flat_map(|s| &mut s.faces).enumerate() {
        let _ = index;
        if chosen.contains(&usage.face) {
            usage.face = usize::MAX;
        } else {
            usage.face = retained_faces[usage.face].unwrap();
        }
    }
    model.shells.iter_mut().for_each(|s| s.faces.retain(|u| u.face != usize::MAX));
    for i in 0..sheet.faces.len() {
        model.shells[shell].faces.push(FaceUse { face: new_face + i, reversed });
    }
    crate::retained_uv_rim::subdivide_and_sew_linear_rims(
        &mut model,
        edge_offset,
        edge_offset..edge_offset + sheet.edges.len(),
        vertex_offset,
    )?;
    let used_edges = model.loops.iter().flat_map(|w| &w.coedges).fold(vec![false; model.edges.len()], |mut used, c| { used[c.edge] = true; used });
    let mut edge_map = vec![None; model.edges.len()];
    let mut edges = Vec::new();
    for (i, edge) in model.edges.iter().enumerate() {
        if used_edges[i] { edge_map[i] = Some(edges.len()); edges.push(edge.clone()); }
    }
    model.edges = edges;
    model.loops.iter_mut().flat_map(|w| &mut w.coedges).for_each(|c| c.edge = edge_map[c.edge].unwrap());
    let used_vertices = model.edges.iter().fold(vec![false; model.vertices.len()], |mut used, e| { used[e.vertices[0]] = true; used[e.vertices[1]] = true; used });
    let mut vertex_map = vec![None; model.vertices.len()];
    let mut vertices = Vec::new();
    for (i, vertex) in model.vertices.iter().enumerate() {
        if used_vertices[i] { vertex_map[i] = Some(vertices.len()); vertices.push(vertex.clone()); }
    }
    model.vertices = vertices;
    model.edges.iter_mut().for_each(|e| e.vertices = e.vertices.map(|v| vertex_map[v].unwrap()));
    model.inherit_topology_ids(&[source]);
    let children = model.1.faces[new_face..].to_vec();
    model.1.lineage.push(TopologyLineageRecord {
        operation: "merge".into(), entity_kind: "face".into(),
        parents: selected.iter().map(|&face| source.1.faces[face]).collect(), children,
    });
    model.refresh_change_set(&[source]);
    validate_staged(&model)?;
    Ok(model)
}

/// Replace one retained face by a certified UV partition. The source is never
/// mutated; a failed lift, shell incidence or boundary proof returns no model.
pub fn replace_face_with_uv_regions(
    source: &Model,
    face: usize,
    regions: &[UvRegion],
    limits: SameSurfaceUvLimits,
) -> Result<Model> {
    source.validate()?;
    let usage = sole_face_use(source, face)?;
    let shell = source.shells.iter().position(|s| s.faces.iter().any(|u| u.face == face))
        .ok_or_else(|| refused("Selected face has no shell"))?;
    let sheet = build_uv_region_sheet(&source.faces[face].surface, regions, usage.reversed, source.tolerance_mm, limits)?;
    replace_faces(source, &[face], shell, usage.reversed, sheet)
}

/// Merge an oriented same-carrier face region inside a retained model. Closed
/// shells stay closed and retain all unselected faces, bodies and inner shells.
pub fn merge_same_surface_uv_faces_with_limits(source: &Model, selected: &[usize], limits: SameSurfaceUvLimits) -> Result<Model> {
    source.validate()?;
    if selected.len() < 2 || selected.iter().any(|&face| face >= source.faces.len()) {
        return Err(refused("Same-carrier merge needs at least two existing faces"));
    }
    let unique = selected.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != selected.len() { return Err(refused("Duplicate selected face")); }
    let first = selected[0];
    let shell = source.shells.iter().position(|s| s.faces.iter().any(|u| u.face == first)).ok_or_else(|| refused("Selected face has no shell"))?;
    let reversed = sole_face_use(source, first)?.reversed;
    if selected.iter().any(|&face| sole_face_use(source, face).ok().is_none_or(|u| u.reversed != reversed)
        || !source.shells[shell].faces.iter().any(|u| u.face == face))
        || selected.iter().any(|&face| source.faces[face].surface != source.faces[first].surface)
    {
        return Err(refused("Merge requires identical carriers and same-sense uses in one shell"));
    }
    let mut current = boolean_same_surface_uv_with_limits(
        &source.faces[first].surface, source, first, &source.faces[first].surface, source, selected[1],
        None, UvBooleanOperation::Union, limits,
    )?;
    for &next in &selected[2..] {
        let staged = build_uv_region_sheet(&source.faces[first].surface, current.regions(), reversed, source.tolerance_mm, limits)?;
        current = boolean_same_surface_uv_with_limits(
            &source.faces[first].surface, &staged, 0, &source.faces[next].surface, source, next,
            None, UvBooleanOperation::Union, limits,
        )?;
    }
    let sheet = build_uv_region_sheet(&source.faces[first].surface, current.regions(), reversed, source.tolerance_mm, limits)?;
    replace_faces(source, selected, shell, reversed, sheet)
}

pub fn merge_same_surface_uv_faces(source: &Model, selected: &[usize]) -> Result<Model> {
    merge_same_surface_uv_faces_with_limits(source, selected, SameSurfaceUvLimits::default())
}
