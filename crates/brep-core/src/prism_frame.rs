//! Rigid coordinates for equally oriented prismatic operands. Coordinate
//! normalization is explicitly bounded numerical work, not a solid certificate.
use super::*;

#[derive(Clone, Debug)]
pub struct Frame {
    pub origin: [f64; 3],
    /// Local X, Y, Z axes expressed in world coordinates, right handed.
    pub axes: [[f64; 3]; 3],
    /// Allowed absolute correction to one coordinate, independent of model tolerance.
    pub roundoff_bound_mm: f64,
    /// Largest single-coordinate correction, not Euclidean point displacement.
    pub max_adjustment_mm: f64,
    /// Number of scalar coordinate values changed during localization.
    pub adjustment_count: usize,
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    (n.is_finite() && n > 0.).then(|| a.map(|x| x / n))
}
fn point(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
fn coordinates(model: &Model) -> impl Iterator<Item = [f64; 3]> + '_ {
    model
        .vertices
        .iter()
        .map(|v| v.point)
        .chain(
            model
                .edges
                .iter()
                .flat_map(|e| e.curve.control_points.iter())
                .map(|p| point(p)),
        )
        .chain(
            model
                .faces
                .iter()
                .flat_map(|f| f.surface.control_points.iter().flatten())
                .map(|p| point(p)),
        )
}
fn map_points(model: &mut Model, transform: impl Fn([f64; 3]) -> [f64; 3]) {
    for vertex in &mut model.vertices {
        vertex.point = transform(vertex.point);
    }
    for edge in &mut model.edges {
        for p in &mut edge.curve.control_points {
            *p = transform(point(p)).to_vec();
        }
    }
    for face in &mut model.faces {
        for p in face.surface.control_points.iter_mut().flatten() {
            *p = transform(point(p)).to_vec();
        }
    }
}
impl Frame {
    fn valid(&self) -> bool {
        self.origin
            .iter()
            .chain(self.axes.iter().flatten())
            .all(|x| x.is_finite())
            && (0..3).all(|i| (dot(self.axes[i], self.axes[i]) - 1.).abs() <= 128. * f64::EPSILON)
            && (0..3).all(|i| {
                (i + 1..3).all(|j| dot(self.axes[i], self.axes[j]).abs() <= 128. * f64::EPSILON)
            })
            && dot(cross(self.axes[0], self.axes[1]), self.axes[2]) > 1. - 128. * f64::EPSILON
    }
    fn to_local(&self, p: [f64; 3]) -> [f64; 3] {
        let d = sub(p, self.origin);
        self.axes.map(|axis| dot(d, axis))
    }
    fn to_world(&self, p: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| {
            self.origin[i] + (0..3).map(|j| p[j] * self.axes[j][i]).sum::<f64>()
        })
    }
    /// Inverse rigid map. Preserve all inherited topology and lineage IDs;
    /// restoring world coordinates never regenerates identity tables.
    pub fn restore(&self, model: &Model) -> Result<Model> {
        if !self.valid() {
            return Err(Error::new(
                "BREP_INVALID_FRAME",
                "Prism frame must be finite, orthonormal and right handed",
            ));
        }
        model.validate()?;
        let mut restored = model.clone();
        map_points(&mut restored, |p| self.to_world(p));
        restored.validate()?;
        Ok(restored)
    }
    fn adjust(&mut self, value: &mut f64, target: f64) -> bool {
        let change = (*value - target).abs();
        if !change.is_finite() || change > self.roundoff_bound_mm {
            return false;
        }
        if change > 0. {
            self.adjustment_count += 1;
            self.max_adjustment_mm = self.max_adjustment_mm.max(change);
            *value = target;
        }
        true
    }
}
#[derive(Clone)]
struct Candidate {
    direction: [f64; 3],
    curved: bool,
}
fn candidates(model: &Model, bound: f64) -> Vec<Candidate> {
    let mut result = Vec::new();
    for face in &model.faces {
        let s = &face.surface;
        for axis in 0..2 {
            let (degree, count, periodic) = if axis == 0 {
                (s.degree_u, s.control_points.len(), s.periodic_u)
            } else {
                (s.degree_v, s.control_points[0].len(), s.periodic_v)
            };
            if degree != 1 || count != 2 || periodic {
                continue;
            }
            let n = if axis == 0 {
                s.control_points[0].len()
            } else {
                s.control_points.len()
            };
            let pairs: Vec<_> = (0..n)
                .map(|i| {
                    if axis == 0 {
                        (
                            point(&s.control_points[0][i]),
                            point(&s.control_points[1][i]),
                            s.weights[0][i],
                            s.weights[1][i],
                        )
                    } else {
                        (
                            point(&s.control_points[i][0]),
                            point(&s.control_points[i][1]),
                            s.weights[i][0],
                            s.weights[i][1],
                        )
                    }
                })
                .collect();
            let vector = sub(pairs[0].1, pairs[0].0);
            if norm(vector) <= 16. * bound
                || pairs
                    .iter()
                    .any(|(a, b, wa, wb)| wa != wb || norm(sub(sub(*b, *a), vector)) > bound)
            {
                continue;
            }
            let Some(mut direction) = unit(vector) else {
                continue;
            };
            let pivot = (0..3)
                .max_by(|&i, &j| {
                    direction[i]
                        .abs()
                        .total_cmp(&direction[j].abs())
                        .then(j.cmp(&i))
                })
                .unwrap();
            if direction[pivot] < 0. {
                direction = direction.map(|x| -x);
            }
            result.push(Candidate {
                direction,
                curved: if axis == 0 {
                    s.degree_v > 1
                } else {
                    s.degree_u > 1
                },
            });
        }
    }
    result
}
fn canonical_levels(mut levels: Vec<f64>, bound: f64) -> Option<Vec<f64>> {
    if levels.iter().any(|z| !z.is_finite()) {
        return None;
    }
    levels.sort_by(f64::total_cmp);
    let mut canonical = Vec::<f64>::new();
    for z in levels {
        if canonical.last().is_none_or(|last| z - last > bound) {
            canonical.push(z);
        }
    }
    // Near-distinct layers must not be collapsed into an apparent common cap.
    // Representatives are minima, so every adjustment remains independently
    // bounded instead of accumulating across a chain of near neighbors.
    if canonical.len() < 2
        || canonical
            .windows(2)
            .any(|pair| pair[1] - pair[0] <= 16. * bound)
    {
        return None;
    }
    Some(canonical)
}
fn normalize_z(frame: &mut Frame, p: &mut [f64], levels: &[f64]) -> bool {
    let Some(target) = levels
        .iter()
        .copied()
        .min_by(|a, b| (a - p[2]).abs().total_cmp(&(b - p[2]).abs()))
    else {
        return false;
    };
    frame.adjust(&mut p[2], target)
}
fn localized(frame: &mut Frame, model: &Model) -> Result<Option<Model>> {
    let mut local = model.clone();
    map_points(&mut local, |p| frame.to_local(p));
    let Some(levels) = canonical_levels(
        local.vertices.iter().map(|v| v.point[2]).collect(),
        frame.roundoff_bound_mm,
    ) else {
        return Ok(None);
    };
    if levels.len() > crate::stepped_prism::MAX_LAYERS + 1 {
        return Ok(None);
    }
    for vertex in &mut local.vertices {
        if !normalize_z(frame, &mut vertex.point, &levels) {
            return Ok(None);
        }
    }
    for edge in &mut local.edges {
        for p in &mut edge.curve.control_points {
            if !normalize_z(frame, p, &levels) {
                return Ok(None);
            }
        }
    }
    for face in &mut local.faces {
        let s = &mut face.surface;
        for p in s.control_points.iter_mut().flatten() {
            if !normalize_z(frame, p, &levels) {
                return Ok(None);
            }
        }
        // Only degree-one extrusion columns may be numerically normalized.
        // Every complete ruled side must use a consistent pair of Z planes;
        // the layered recognizer checks all remaining caps and interfaces.
        for axis in 0..2 {
            let (degree, count) = if axis == 0 {
                (s.degree_u, s.control_points.len())
            } else {
                (s.degree_v, s.control_points[0].len())
            };
            if degree != 1 || count != 2 {
                continue;
            }
            let n = if axis == 0 {
                s.control_points[0].len()
            } else {
                s.control_points.len()
            };
            let is_vertical = (0..n).all(|i| {
                let (a, b, wa, wb) = if axis == 0 {
                    (
                        &s.control_points[0][i],
                        &s.control_points[1][i],
                        s.weights[0][i],
                        s.weights[1][i],
                    )
                } else {
                    (
                        &s.control_points[i][0],
                        &s.control_points[i][1],
                        s.weights[i][0],
                        s.weights[i][1],
                    )
                };
                a[2] != b[2]
                    && wa == wb
                    && (0..2).all(|j| (a[j] - b[j]).abs() <= frame.roundoff_bound_mm)
            });
            if !is_vertical {
                continue;
            }
            for i in 0..n {
                for coordinate in 0..2 {
                    let (a, b) = if axis == 0 {
                        (
                            s.control_points[0][i][coordinate],
                            s.control_points[1][i][coordinate],
                        )
                    } else {
                        (
                            s.control_points[i][0][coordinate],
                            s.control_points[i][1][coordinate],
                        )
                    };
                    let target = a + (b - a) * 0.5;
                    if axis == 0 {
                        if !frame.adjust(&mut s.control_points[0][i][coordinate], target)
                            || !frame.adjust(&mut s.control_points[1][i][coordinate], target)
                        {
                            return Ok(None);
                        }
                    } else if !frame.adjust(&mut s.control_points[i][0][coordinate], target)
                        || !frame.adjust(&mut s.control_points[i][1][coordinate], target)
                    {
                        return Ok(None);
                    }
                }
            }
        }
    }
    // Structural recognition checks all caps, carriers, trims, side nets and
    // incidence after the explicitly recorded numerical normalization.
    match crate::stepped_prism::recognize(&local) {
        Ok(Some(_)) => Ok(Some(local)),
        Ok(None) => Ok(None),
        Err(_) => Ok(None),
    }
}
fn normalize_common_z(frame: &mut Frame, a: &mut Model, b: &mut Model) -> bool {
    let Some(canonical) = canonical_levels(
        a.vertices
            .iter()
            .chain(&b.vertices)
            .map(|v| v.point[2])
            .collect(),
        frame.roundoff_bound_mm,
    ) else {
        return false;
    };
    for model in [a, b] {
        for vertex in &mut model.vertices {
            if !normalize_z(frame, &mut vertex.point, &canonical) {
                return false;
            }
        }
        for edge in &mut model.edges {
            for p in &mut edge.curve.control_points {
                if !normalize_z(frame, p, &canonical) {
                    return false;
                }
            }
        }
        for face in &mut model.faces {
            for p in face.surface.control_points.iter_mut().flatten() {
                if !normalize_z(frame, p, &canonical) {
                    return false;
                }
            }
        }
    }
    true
}
fn record_total_adjustments(
    frame: &mut Frame,
    originals: [&Model; 2],
    localized: [&Model; 2],
) -> bool {
    frame.adjustment_count = 0;
    frame.max_adjustment_mm = 0.;
    for (original, local) in originals.into_iter().zip(localized) {
        for (source, actual) in coordinates(original).zip(coordinates(local)) {
            let before = frame.to_local(source);
            for i in 0..3 {
                let difference = (before[i] - actual[i]).abs();
                if difference > frame.roundoff_bound_mm {
                    return false;
                }
                if difference > 0. {
                    frame.adjustment_count += 1;
                    frame.max_adjustment_mm = frame.max_adjustment_mm.max(difference);
                }
            }
        }
    }
    true
}
/// Find a deterministic common rigid frame, preferring curved ruled sides over
/// the ambiguous extrusion directions of boxes. Both complete operands must
/// then pass full prism or layered-prism recognition. Authored perturbations indistinguishable from
/// roundoff can lie inside the reported bound; this is not exact certification.
pub fn localize(a: &Model, b: &Model) -> Result<Option<(Model, Model, Frame)>> {
    a.validate()?;
    b.validate()?;
    if a.vertices.is_empty() || b.vertices.is_empty() {
        return Ok(None);
    }
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let mut scale: f64 = 1.;
    for p in coordinates(a).chain(coordinates(b)) {
        for i in 0..3 {
            bounds[0][i] = bounds[0][i].min(p[i]);
            bounds[1][i] = bounds[1][i].max(p[i]);
            scale = scale.max(p[i].abs());
        }
    }
    let origin = std::array::from_fn(|i| bounds[0][i] + (bounds[1][i] - bounds[0][i]) * 0.5);
    let bound = 64. * f64::EPSILON * scale;
    let mut directions = candidates(a, bound);
    directions.extend(candidates(b, bound));
    directions.sort_by(|a, b| {
        b.curved.cmp(&a.curved).then_with(|| {
            (0..3)
                .map(|i| a.direction[i].total_cmp(&b.direction[i]))
                .find(|x| !x.is_eq())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    let mut tried = Vec::<[f64; 3]>::new();
    for candidate in directions {
        let z = candidate.direction;
        if tried
            .iter()
            .any(|d| norm(sub(*d, z)) <= 128. * f64::EPSILON)
        {
            continue;
        }
        tried.push(z);
        let helper = (0..3)
            .min_by(|&i, &j| z[i].abs().total_cmp(&z[j].abs()).then(i.cmp(&j)))
            .unwrap();
        let mut basis = [0.; 3];
        basis[helper] = 1.;
        let Some(x) = unit(sub(basis, z.map(|w| w * z[helper]))) else {
            continue;
        };
        let y = cross(z, x);
        let mut frame = Frame {
            origin,
            axes: [x, y, z],
            roundoff_bound_mm: bound,
            max_adjustment_mm: 0.,
            adjustment_count: 0,
        };
        if !frame.valid() {
            continue;
        }
        let Some(mut local_a) = localized(&mut frame, a)? else {
            continue;
        };
        let Some(mut local_b) = localized(&mut frame, b)? else {
            continue;
        };
        if !normalize_common_z(&mut frame, &mut local_a, &mut local_b)
            || !record_total_adjustments(&mut frame, [a, b], [&local_a, &local_b])
        {
            continue;
        }
        local_a.validate()?;
        local_b.validate()?;
        return Ok(Some((local_a, local_b, frame)));
    }
    Ok(None)
}
