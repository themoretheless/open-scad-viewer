//! Bounded regularized Boolean over analytic XY cells and exact Z slabs.
//! Retained source curves author every cell. Complete opposite cell faces are
//! cancelled; topology is sewn only across those proven shared faces.
use super::*;

type Profile = Vec<Vec<Curve>>;
pub(crate) const MAX_LAYERS: usize = 8;
const MAX_CELLS: usize = 32;
const MAX_PROFILES: usize = 16;

#[derive(Clone)]
pub(crate) struct Layer {
    low: f64,
    high: f64,
    profile: Profile,
}
fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}
fn ambiguous(message: impl Into<String>) -> Error {
    Error::new("BREP_AMBIGUOUS_PLANAR_TRIM", message)
}
fn limit() -> Error {
    Error::new(
        "BREP_RESOURCE_LIMIT",
        "Stepped prism exceeds 8 layers, 16 profiles, 32 planar cells, 256 profile curves/partition queries, 2048 partition fragments, or existing B-rep limits",
    )
}
fn area(profile: &Profile, tol: f64) -> Result<f64> {
    profile.iter().try_fold(0., |sum, wire| {
        Ok(sum + planar_trim::signed_area(wire, tol)?)
    })
}
fn difference(a: &Profile, b: &Profile, tol: f64) -> Result<Profile> {
    planar_trim::boolean(a, b, "difference", tol)
}
fn equal(a: &Profile, b: &Profile, tol: f64) -> Result<bool> {
    Ok(difference(a, b, tol)?.is_empty() && difference(b, a, tol)?.is_empty())
}
fn normalized(mut c: Curve) -> Curve {
    let [a, b] = c.domain();
    for k in &mut c.knots {
        *k = (*k - a) / (b - a);
    }
    c
}
fn xy(mut c: Curve) -> Curve {
    for p in &mut c.control_points {
        p.pop();
    }
    normalized(c)
}
fn curve_key(c: &Curve) -> Result<(String, bool)> {
    let forward = value_codec::to_string(c).map_err(|e| unsupported(e.to_string()))?;
    let reverse = value_codec::to_string(&c.reverse()?).map_err(|e| unsupported(e.to_string()))?;
    Ok(if forward <= reverse {
        (forward, false)
    } else {
        (reverse, true)
    })
}
fn same_curve(a: &Curve, b: &Curve, epsilon: f64) -> bool {
    let a = normalized(a.clone());
    let b = normalized(b.clone());
    a.degree == b.degree
        && a.periodic == b.periodic
        && a.knots == b.knots
        && a.control_points.len() == b.control_points.len()
        && a.weights.iter().zip(&b.weights).all(|(x, y)| {
            (x / a.weights[0] - y / b.weights[0]).abs()
                <= 128. * f64::EPSILON * (x / a.weights[0]).abs().max(1.)
        })
        && a.control_points
            .iter()
            .zip(&b.control_points)
            .all(|(p, q)| {
                p.len() == q.len() && p.iter().zip(q).all(|(x, y)| (x - y).abs() <= epsilon)
            })
}

/// Slice connectivity comes from complete retained curves. Ambiguous endpoint
/// fans are refused; no arbitrary ordering converts a point contact into a body.
fn sew_profile(mut curves: Vec<Curve>, tolerance: f64) -> Result<Profile> {
    if curves.is_empty() {
        return Ok(vec![]);
    }
    let scale = curves
        .iter()
        .flat_map(|c| {
            c.control_points
                .iter()
                .map(move |p| (p[0] - c.control_points[0][0]).hypot(p[1] - c.control_points[0][1]))
        })
        .fold(1., f64::max);
    let epsilon = scale * 4096. * f64::EPSILON;
    if epsilon > tolerance {
        return Err(ambiguous("Layer slice exceeds coordinate precision"));
    }
    let near = |a: &[f64], b: &[f64]| (a[0] - b[0]).hypot(a[1] - b[1]) <= epsilon;
    let mut loops = Vec::new();
    while let Some(first) = curves.pop() {
        let start = first.evaluate(0.)?.point;
        let mut end = first.evaluate(1.)?.point;
        let mut wire = vec![first];
        while !near(&end, &start) {
            let candidates = curves
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    c.control_points
                        .first()
                        .filter(|p| near(p, &end))
                        .map(|_| i)
                })
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                return Err(ambiguous(
                    "Layer has an unmatched or branching boundary endpoint",
                ));
            }
            let next = curves.remove(candidates[0]);
            end = next.evaluate(1.)?.point;
            wire.push(next);
        }
        loops.push(wire);
    }
    planar_trim::validate(&loops, tolerance)?;
    Ok(loops)
}

fn cap_profile(model: &Model, usage: &FaceUse, upward: bool) -> Result<Profile> {
    let face = &model.faces[usage.face];
    std::iter::once(&face.outer)
        .chain(&face.holes)
        .map(|&wire| {
            let mut result = model.loops[wire]
                .coedges
                .iter()
                .map(|c| prism::directed_edge(model, c).map(xy))
                .collect::<Result<Vec<_>>>()?;
            // A cap region is represented with material left, independent of which
            // physical side of the solid this face bounds.
            if usage.reversed == upward {
                result.reverse();
                result = result
                    .into_iter()
                    .map(|c| c.reverse())
                    .collect::<Result<_>>()?;
            }
            Ok(result)
        })
        .collect()
}
fn union_disjoint(regions: &[Profile], tolerance: f64) -> Result<Profile> {
    let mut result = vec![];
    for profile in regions {
        planar_trim::validate(profile, tolerance)?;
        if !planar_trim::boolean(&result, profile, "intersection", tolerance)?.is_empty() {
            return Err(unsupported(
                "Layer caps contain overlapping or duplicate positive-area regions",
            ));
        }
        result = planar_trim::boolean(&result, profile, "union", tolerance)?;
    }
    Ok(result)
}

fn cap_volume(model: &Model, faces: &[FaceUse], origin: f64) -> Result<(f64, f64)> {
    let mut volume = 0.;
    let mut magnitude = 0.;
    for usage in faces {
        if let Some(z) = prism::planar_cap_z(&model.faces[usage.face].surface) {
            let up = prism::cap_plane_is_proven(model, usage, z, true)?;
            let term = (z - origin)
                * area(&cap_profile(model, usage, up)?, model.tolerance_mm)?
                * if up { 1. } else { -1. };
            volume += term;
            magnitude += term.abs();
        }
    }
    Ok((volume, magnitude * 4096. * f64::EPSILON))
}
/// A structural proof of the finite layered envelope, including cap coverage.
/// It accepts serialized/reordered native topology, not construction metadata.
pub(crate) fn recognize(model: &Model) -> Result<Option<Vec<Layer>>> {
    model.validate()?;
    if let Some(p) = prism::recognize(model)? {
        planar_trim::validate(&p.loops, model.tolerance_mm)?;
        return Ok(Some(vec![Layer {
            low: p.z_min,
            high: p.z_max,
            profile: p.loops,
        }]));
    }
    if model.is_empty() {
        return Ok(Some(vec![]));
    }
    let mut sides = Vec::<(usize, f64, f64, Curve)>::new();
    let mut caps = Vec::<(f64, bool, Profile)>::new();
    let mut levels = Vec::new();
    let shell_owners = model
        .bodies
        .iter()
        .enumerate()
        .flat_map(|(i, b)| {
            std::iter::once(&b.outer_shell)
                .chain(&b.inner_shells)
                .map(move |&s| (s, i))
        })
        .collect::<BTreeMap<_, _>>();
    for (shell_id, shell) in model.shells.iter().enumerate() {
        let Some(&owner) = shell_owners.get(&shell_id) else {
            return Ok(None);
        };
        for usage in &shell.faces {
            let face = &model.faces[usage.face];
            if let Some(z) = prism::planar_cap_z(&face.surface) {
                let up = prism::cap_plane_is_proven(model, usage, z, true)?;
                if !up && !prism::cap_plane_is_proven(model, usage, z, false)? {
                    return Ok(None);
                }
                caps.push((z, up, cap_profile(model, usage, up)?));
                levels.push(z);
                continue;
            }
            let low = face
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| p[2])
                .fold(f64::INFINITY, f64::min);
            let high = face
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max);
            if low >= high || !prism::side_is_proven(model, usage, low, high)? {
                return Ok(None);
            }
            let bottom = model.loops[face.outer]
                .coedges
                .iter()
                .filter(|c| {
                    model.edges[c.edge]
                        .curve
                        .control_points
                        .iter()
                        .all(|p| p[2] == low)
                })
                .collect::<Vec<_>>();
            if bottom.len() != 1 {
                return Ok(None);
            }
            let mut curve = prism::directed_edge(model, bottom[0])?;
            if usage.reversed {
                curve = curve.reverse()?;
            }
            sides.push((owner, low, high, xy(curve)));
            levels.extend([low, high]);
        }
        let origin = shell
            .faces
            .iter()
            .flat_map(|u| model.faces[u.face].surface.control_points.iter().flatten())
            .map(|p| p[2])
            .fold(f64::INFINITY, f64::min);
        let (volume, error) = cap_volume(model, &shell.faces, origin)?;
        if volume.abs() <= error || !volume.is_finite() {
            return Err(ambiguous(
                "Declared layered shell has unresolved oriented cap volume",
            ));
        }
        if (volume > 0.) != (model.bodies[owner].outer_shell == shell_id) {
            return Err(unsupported(
                "Layered prism shell role disagrees with its oriented boundary",
            ));
        }
    }
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    if levels.len() > MAX_LAYERS + 1 {
        return Err(limit());
    }
    let mut layers = Vec::new();
    for bounds in levels.windows(2) {
        let mut profile = Vec::new();
        for owner in 0..model.bodies.len() {
            profile.extend(sew_profile(
                sides
                    .iter()
                    .filter(|(o, a, b, _)| *o == owner && *a <= bounds[0] && *b >= bounds[1])
                    .map(|(_, _, _, c)| c.clone())
                    .collect(),
                model.tolerance_mm,
            )?);
        }
        planar_trim::validate(&profile, model.tolerance_mm)?;
        layers.push(Layer {
            low: bounds[0],
            high: bounds[1],
            profile,
        });
    }
    for (i, &z) in levels.iter().enumerate() {
        let below = if i == 0 {
            vec![]
        } else {
            layers[i - 1].profile.clone()
        };
        let above = layers.get(i).map_or(vec![], |l| l.profile.clone());
        for (up, expected) in [
            (true, difference(&below, &above, model.tolerance_mm)?),
            (false, difference(&above, &below, model.tolerance_mm)?),
        ] {
            let actual = union_disjoint(
                &caps
                    .iter()
                    .filter(|(h, u, _)| *h == z && *u == up)
                    .map(|(_, _, p)| p.clone())
                    .collect::<Vec<_>>(),
                model.tolerance_mm,
            )?;
            if !equal(&actual, &expected, model.tolerance_mm)? {
                return Err(unsupported(
                    "Layered prism cap regions do not equal the adjacent slab difference",
                ));
            }
        }
    }
    Ok(Some(layers))
}

#[derive(Clone)]
struct Cell {
    profile: Profile,
    membership: u16,
}
fn partition(profiles: &[Profile], tolerance: f64) -> Result<Vec<Cell>> {
    if profiles.len() > MAX_PROFILES {
        return Err(limit());
    }
    if profiles.iter().flatten().map(Vec::len).sum::<usize>() > 256 {
        return Err(limit());
    }
    let mut cells = Vec::<Cell>::new();
    let mut queries = 0;
    let mut fragments = 0;
    for (i, profile) in profiles.iter().enumerate() {
        if profile.is_empty() {
            continue;
        }
        let mut remaining = profile.clone();
        let mut next = Vec::new();
        for cell in cells {
            queries += 3;
            if queries > 256 {
                return Err(limit());
            }
            let common = planar_trim::boolean(&cell.profile, profile, "intersection", tolerance)?;
            let outside = difference(&cell.profile, profile, tolerance)?;
            remaining = difference(&remaining, &cell.profile, tolerance)?;
            fragments += common
                .iter()
                .chain(&outside)
                .chain(&remaining)
                .map(Vec::len)
                .sum::<usize>();
            if fragments > 2048 {
                return Err(limit());
            }
            if !common.is_empty() {
                next.push(Cell {
                    profile: common,
                    membership: cell.membership | (1 << i),
                });
            }
            if !outside.is_empty() {
                next.push(Cell {
                    profile: outside,
                    membership: cell.membership,
                });
            }
            if next.len() > MAX_CELLS {
                return Err(limit());
            }
        }
        if !remaining.is_empty() {
            next.push(Cell {
                profile: remaining,
                membership: 1 << i,
            });
        }
        if next.len() > MAX_CELLS {
            return Err(limit());
        }
        cells = next;
    }
    let common = planar_trim::conform_profiles(
        &cells.iter().map(|c| c.profile.clone()).collect::<Vec<_>>(),
        tolerance,
    )?;
    for (cell, profile) in cells.iter_mut().zip(common) {
        cell.profile = profile;
    }
    Ok(cells)
}

#[derive(Clone)]
struct Dsu(Vec<usize>);
impl Dsu {
    fn new(n: usize) -> Self {
        Self((0..n).collect())
    }
    fn root(&self, mut i: usize) -> usize {
        while self.0[i] != i {
            i = self.0[i];
        }
        i
    }
    fn join(&mut self, a: usize, b: usize) {
        let a = self.root(a);
        let b = self.root(b);
        self.0[b] = a;
    }
}
struct FaceRecord {
    usage: FaceUse,
    owner: usize,
    key: String,
    direction: bool,
}
fn record_key(model: &Model, usage: &FaceUse) -> Result<(String, bool)> {
    // Only canonical prism::extrude parts reach this constructor helper:
    // cap carriers point +Z; side U follows the profile and V increases Z.
    // This key is not a generic equivalence test for arbitrary parameterizations.
    let face = &model.faces[usage.face];
    if let Some(z) = prism::planar_cap_z(&face.surface) {
        let mut wires = Vec::new();
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            let mut keys = model.loops[wire]
                .coedges
                .iter()
                .map(|c| curve_key(&model.edges[c.edge].curve).map(|k| k.0))
                .collect::<Result<Vec<_>>>()?;
            keys.sort();
            wires.push(keys.join(";"));
        }
        wires.sort();
        Ok((format!("cap:{z:?}:{}", wires.join("|")), usage.reversed))
    } else {
        let surface = &face.surface;
        let profile = Curve {
            degree: surface.degree_u,
            knots: surface.knots_u.clone(),
            control_points: surface
                .control_points
                .iter()
                .map(|row| row[0][..2].to_vec())
                .collect(),
            weights: surface.weights.iter().map(|row| row[0]).collect(),
            periodic: surface.periodic_u,
        };
        let (key, reverse) = curve_key(&profile)?;
        Ok((
            format!(
                "side:{:?}:{:?}:{key}",
                surface.control_points[0][0][2], surface.control_points[0][1][2]
            ),
            reverse ^ usage.reversed,
        ))
    }
}
fn append(
    target: &mut Model,
    part: &Model,
    records: &mut Vec<FaceRecord>,
    owners: &mut usize,
) -> Result<()> {
    let v = target.vertices.len();
    let e = target.edges.len();
    let l = target.loops.len();
    let f = target.faces.len();
    target.vertices.extend(part.vertices.iter().cloned());
    target.edges.extend(part.edges.iter().cloned().map(|mut x| {
        x.vertices = x.vertices.map(|i| i + v);
        x
    }));
    target.loops.extend(part.loops.iter().cloned().map(|mut x| {
        for c in &mut x.coedges {
            c.edge += e;
        }
        x
    }));
    target.faces.extend(part.faces.iter().cloned().map(|mut x| {
        x.outer += l;
        for h in &mut x.holes {
            *h += l;
        }
        x
    }));
    for body in &part.bodies {
        for use_ in &part.shells[body.outer_shell].faces {
            let (key, direction) = record_key(part, use_)?;
            records.push(FaceRecord {
                usage: FaceUse {
                    face: use_.face + f,
                    reversed: use_.reversed,
                },
                owner: *owners,
                key,
                direction,
            });
        }
        *owners += 1;
    }
    if target.faces.len() > 256
        || target.vertices.len() + target.edges.len() + target.loops.len() + target.faces.len()
            > 4096
    {
        return Err(limit());
    }
    Ok(())
}
fn face_edges(model: &Model, face: usize) -> Vec<usize> {
    let face = &model.faces[face];
    std::iter::once(&face.outer)
        .chain(&face.holes)
        .flat_map(|&l| model.loops[l].coedges.iter().map(|c| c.edge))
        .collect()
}

fn assemble(parts: Vec<Model>, tolerance: f64) -> Result<Model> {
    let mut source = Model::empty(tolerance)?;
    let mut records = Vec::new();
    let mut owners = 0;
    for part in parts {
        append(&mut source, &part, &mut records, &mut owners)?;
    }
    if records.is_empty() {
        return Ok(source);
    }
    let anchor = source.vertices[0].point;
    let scale = source
        .vertices
        .iter()
        .flat_map(|v| (0..3).map(move |i| (v.point[i] - anchor[i]).abs()))
        .fold(1., f64::max);
    let epsilon = scale * 4096. * f64::EPSILON;
    if epsilon > tolerance {
        return Err(ambiguous("Cell sewing exceeds coordinate precision"));
    }
    let mut vertices = Dsu::new(source.vertices.len());
    let mut edges = Dsu::new(source.edges.len());
    let mut bodies = Dsu::new(owners);
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (i, r) in records.iter().enumerate() {
        groups.entry(r.key.clone()).or_default().push(i);
    }
    let mut cancelled = vec![false; records.len()];
    for group in groups.values().filter(|v| v.len() > 1) {
        if group.len() != 2 || records[group[0]].direction == records[group[1]].direction {
            return Err(unsupported(
                "Cell decomposition has duplicate or nonmanifold coincident faces",
            ));
        }
        let a = &records[group[0]];
        let b = &records[group[1]];
        let ae = face_edges(&source, a.usage.face);
        let be = face_edges(&source, b.usage.face);
        if ae.len() != be.len() {
            return Err(unsupported(
                "Shared cell faces have incompatible edge partitions",
            ));
        }
        let mut used = BTreeSet::new();
        for x in ae {
            let mut candidates = Vec::new();
            for &y in &be {
                if used.contains(&y) {
                    continue;
                }
                let ca = &source.edges[x].curve;
                let cb = &source.edges[y].curve;
                if same_curve(ca, cb, epsilon) {
                    candidates.push((y, false));
                } else if same_curve(ca, &cb.reverse()?, epsilon) {
                    candidates.push((y, true));
                }
            }
            if candidates.len() != 1 {
                return Err(ambiguous("Shared cell edge parameterization is unresolved"));
            }
            let (y, reverse) = candidates[0];
            used.insert(y);
            for i in 0..2 {
                vertices.join(
                    source.edges[x].vertices[i],
                    source.edges[y].vertices[if reverse { 1 - i } else { i }],
                );
            }
            edges.join(x, y);
        }
        bodies.join(a.owner, b.owner);
        cancelled[group[0]] = true;
        cancelled[group[1]] = true;
    }
    let mut model = Model::empty(tolerance)?;
    let mut vertex_ids = BTreeMap::new();
    let mut edge_ids = BTreeMap::new();
    let mut uses = Vec::new();
    let mut face_owners = Vec::new();
    for (i, r) in records.iter().enumerate().filter(|(i, _)| !cancelled[*i]) {
        let _ = i;
        let source_face = &source.faces[r.usage.face];
        let mut wires = Vec::new();
        for &wire in std::iter::once(&source_face.outer).chain(&source_face.holes) {
            let mut coedges = Vec::new();
            for c in &source.loops[wire].coedges {
                let root = edges.root(c.edge);
                let id = if let Some(&id) = edge_ids.get(&root) {
                    id
                } else {
                    let edge = &source.edges[root];
                    let mut ids = [0; 2];
                    for k in 0..2 {
                        let vertex = vertices.root(edge.vertices[k]);
                        ids[k] = *vertex_ids.entry(vertex).or_insert_with(|| {
                            let id = model.vertices.len();
                            model.vertices.push(source.vertices[vertex].clone());
                            id
                        });
                    }
                    let id = model.edges.len();
                    model.edges.push(Edge {
                        vertices: ids,
                        curve: edge.curve.clone(),
                        degenerate: false,
                    });
                    edge_ids.insert(root, id);
                    id
                };
                let forward = vertices.root(source.edges[c.edge].vertices[0])
                    == vertices.root(source.edges[root].vertices[0]);
                coedges.push(Coedge {
                    edge: id,
                    reversed: c.reversed ^ !forward,
                    pcurve: c.pcurve.clone(),
                });
            }
            wires.push(model.loops.len());
            model.loops.push(Loop { coedges });
        }
        let face = model.faces.len();
        model.faces.push(Face {
            surface: source_face.surface.clone(),
            outer: wires[0],
            holes: wires[1..].to_vec(),
        });
        uses.push(FaceUse {
            face,
            reversed: r.usage.reversed,
        });
        face_owners.push(bodies.root(r.owner));
    }
    let mut face_sets = Dsu::new(uses.len());
    let mut incidence = BTreeMap::<usize, Vec<usize>>::new();
    for use_ in &uses {
        for edge in face_edges(&model, use_.face) {
            incidence.entry(edge).or_default().push(use_.face);
        }
    }
    for faces in incidence.values() {
        if faces.len() != 2 {
            return Err(unsupported(
                "Stepped boundary has an unmatched or nonmanifold edge",
            ));
        }
        if face_owners[faces[0]] != face_owners[faces[1]] {
            return Err(unsupported(
                "Point or edge contact cannot merge cell bodies",
            ));
        }
        face_sets.join(faces[0], faces[1]);
    }
    let mut components = BTreeMap::<usize, Vec<FaceUse>>::new();
    for use_ in uses {
        components
            .entry(face_sets.root(use_.face))
            .or_default()
            .push(use_);
    }
    let mut shells = BTreeMap::<usize, (Option<usize>, Vec<usize>)>::new();
    for faces in components.into_values() {
        let owner = face_owners[faces[0].face];
        let (volume, error) = cap_volume(&model, &faces, anchor[2])?;
        if volume.abs() <= error || !volume.is_finite() {
            return Err(ambiguous(
                "Shell orientation has unresolved signed cap volume",
            ));
        }
        let shell = model.shells.len();
        model.shells.push(Shell {
            faces,
            closed: true,
        });
        let entry = shells.entry(owner).or_default();
        if volume > 0. {
            if entry.0.replace(shell).is_some() {
                return Err(unsupported(
                    "Connected cells produced multiple outer shells",
                ));
            }
        } else {
            entry.1.push(shell);
        }
    }
    for (outer, inner_shells) in shells.into_values() {
        model.bodies.push(Body {
            outer_shell: outer.ok_or_else(|| unsupported("Cell body has no outward shell"))?,
            inner_shells,
        });
    }
    model.rebuild_topology_ids();
    model.validate()?;
    // Also prove complete geometric boundary coverage after cancellation. This
    // rejects a pair of individually closed cells with an uncancelled interior.
    if recognize(&model)?.is_none() {
        return Err(unsupported(
            "Assembled boundary left the layered prism envelope",
        ));
    }
    Ok(model)
}

pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    let Some(la) = recognize(a)? else {
        return Ok(None);
    };
    let Some(lb) = recognize(b)? else {
        return Ok(None);
    };
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let profiles = la
        .iter()
        .chain(&lb)
        .map(|l| l.profile.clone())
        .collect::<Vec<_>>();
    let cells = partition(&profiles, tolerance)?;
    let mut levels = la
        .iter()
        .chain(&lb)
        .flat_map(|l| [l.low, l.high])
        .collect::<Vec<_>>();
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    if levels.len() > MAX_LAYERS + 1 {
        return Err(limit());
    }
    let mut parts = Vec::new();
    let mut face_count = 0;
    for bounds in levels.windows(2) {
        for cell in &cells {
            let inside = |layers: &[Layer], offset: usize| {
                layers.iter().enumerate().any(|(i, l)| {
                    l.low <= bounds[0]
                        && l.high >= bounds[1]
                        && cell.membership & (1 << (offset + i)) != 0
                })
            };
            let ia = inside(&la, 0);
            let ib = inside(&lb, la.len());
            let selected = match operation {
                "union" => ia || ib,
                "difference" => ia && !ib,
                "intersection" => ia && ib,
                "xor" => ia != ib,
                _ => {
                    return Err(Error::new(
                        "BREP_INVALID_OPERATION",
                        "Unsupported stepped Boolean operation",
                    ));
                }
            };
            if selected {
                if bounds[1] - bounds[0] < 1e-5 {
                    return Err(unsupported(
                        "Stepped prism requires each occupied Z slab to be at least 0.00001 mm high",
                    ));
                }
                let part = prism::extrude(&cell.profile, bounds[0], bounds[1])?;
                face_count += part.faces.len();
                if face_count > 256 {
                    return Err(limit());
                }
                parts.push(part);
            }
        }
    }
    let mut result = assemble(parts, tolerance)?;
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(Some(result))
}
