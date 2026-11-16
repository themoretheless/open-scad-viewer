//! Exact source-bound axis material chord. Proposals are not certificates.
use super::*;
use nurbs_core::original_graph_transition::{convex_uv_strict_contains, polynomial_plane_point};
pub struct AxisMaterialChord<'a> {
    body: &'a NativeVolume,
    axis: usize,
    faces: [usize; 2],
    uv: [[Q; 2]; 2],
    points: [[Q; 3]; 2],
    length: Q,
    excluded_faces: Vec<usize>,
}
impl<'a> AxisMaterialChord<'a> {
    pub fn body(&self) -> &'a NativeVolume {
        self.body
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn endpoint_uv(&self) -> &[[Q; 2]; 2] {
        &self.uv
    }
    pub fn endpoints(&self) -> &[[Q; 3]; 2] {
        &self.points
    }
    pub fn length_mm(&self) -> &Q {
        &self.length
    }
    pub fn excluded_faces(&self) -> &[usize] {
        &self.excluded_faces
    }
}
fn exact_axis(normal: [[f64; 2]; 3]) -> Option<(usize, i8)> {
    for axis in 0..3 {
        if (0..3).filter(|&k| k != axis).all(|k| normal[k] == [0., 0.]) {
            if normal[axis] == [-1., -1.] {
                return Some((axis, -1));
            }
            if normal[axis] == [1., 1.] {
                return Some((axis, 1));
            }
        }
    }
    None
}
/// Two imported coordinate planes facing each other along one axis, ordered
/// so that `faces[0]` lies at `low` with outward sign -1.
struct OppositePlanes<'f> {
    axis: usize,
    low: Q,
    high: Q,
    faces: [usize; 2],
    surfaces: [&'f ImportedFace; 2],
}
fn midpoint(lo: Q, hi: Q) -> Q {
    (lo + hi) / Q::from_integer(2.into())
}
/// Chord endpoints at the midpoint of the off-axis support-box overlap.
fn midpoint_chord(
    bounds: [&[[Q; 2]; 3]; 2],
    axis: usize,
    low: &Q,
    high: &Q,
) -> Option<[[Q; 3]; 2]> {
    let [ba, bz] = bounds;
    let mut points: [[_; 3]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    for k in 0..3 {
        if k == axis {
            points[0][k] = low.clone();
            points[1][k] = high.clone();
        } else {
            let lo = ba[k][0].clone().max(bz[k][0].clone());
            let hi = ba[k][1].clone().min(bz[k][1].clone());
            if lo >= hi {
                return None;
            }
            let value = midpoint(lo, hi);
            points[0][k] = value.clone();
            points[1][k] = value;
        }
    }
    Some(points)
}
/// Exact UV lifts of both endpoints, strictly inside their imported charts.
fn lift_endpoints(
    surfaces: [&ImportedFace; 2],
    axis: usize,
    points: &[[Q; 3]; 2],
    b: &mut Budget,
) -> Result<Option<[[Q; 2]; 2]>> {
    let mut uv: [[Q; 2]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    for end in 0..2 {
        let Some(p) = polynomial_plane_point(surfaces[end].source(), axis, points[end].clone(), b)?
        else {
            return Ok(None);
        };
        let lines = surfaces[end]
            .uses
            .iter()
            .map(|u| u.uv.clone())
            .collect::<Vec<_>>();
        if !convex_uv_strict_contains(&lines, &p, b)? {
            return Ok(None);
        }
        uv[end] = p;
    }
    Ok(Some(uv))
}
impl<'a> AlignedWallCoverage<'a> {
    fn opposite_planes(&self, pair: [usize; 2]) -> Option<OppositePlanes<'a>> {
        let [i, j] = pair;
        if i == j {
            return None;
        }
        let top = self.body().topology();
        let (FaceGeometry::Imported(a), FaceGeometry::Imported(z)) =
            (&top.faces[i].surface, &top.faces[j].surface)
        else {
            return None;
        };
        let (Some(na), Some(nz)) = (self.frames[i].normal(), self.frames[j].normal()) else {
            return None;
        };
        let (Some((axis, sa)), Some((other, sz))) = (exact_axis(na), exact_axis(nz)) else {
            return None;
        };
        if axis != other || sa == sz {
            return None;
        }
        let ba = self.frames[i].bounds();
        let bz = self.frames[j].bounds();
        if ba[axis][0] != ba[axis][1] || bz[axis][0] != bz[axis][1] {
            return None;
        }
        let (low, high, faces, surfaces, signs) = if ba[axis][0] < bz[axis][0] {
            (
                ba[axis][0].clone(),
                bz[axis][0].clone(),
                [i, j],
                [&**a, &**z],
                [sa, sz],
            )
        } else {
            (
                bz[axis][0].clone(),
                ba[axis][0].clone(),
                [j, i],
                [&**z, &**a],
                [sz, sa],
            )
        };
        if low >= high || signs != [-1, 1] {
            return None;
        }
        Some(OppositePlanes {
            axis,
            low,
            high,
            faces,
            surfaces,
        })
    }
    /// Every other boundary face, when each one's support box misses the chord.
    fn excluded_faces(&self, planes: &OppositePlanes<'_>, point: &[Q; 3]) -> Option<Vec<usize>> {
        let axis = planes.axis;
        let mut excluded = Vec::new();
        let face_count = self.body().topology().faces.len();
        if self.frames.len() < face_count {
            return None;
        }
        for (k, frame) in self.frames.iter().enumerate().take(face_count) {
            if planes.faces.contains(&k) {
                continue;
            }
            let bounds = frame.bounds();
            let off_axis = (0..3)
                .filter(|&d| d != axis)
                .any(|d| point[d] < bounds[d][0] || point[d] > bounds[d][1]);
            let off_segment = bounds[axis][1] < planes.low || bounds[axis][0] > planes.high;
            if !(off_axis || off_segment) {
                return None;
            }
            excluded.push(k);
        }
        Some(excluded)
    }
    /// The admitted chord of one face pair, if every exact check passes.
    fn pair_witness(
        &self,
        pair: [usize; 2],
        b: &mut Budget,
    ) -> Result<Option<AxisMaterialChord<'a>>> {
        let Some(planes) = self.opposite_planes(pair) else {
            return Ok(None);
        };
        let bounds = planes.faces.map(|f| self.frames[f].bounds());
        let Some(points) = midpoint_chord(bounds, planes.axis, &planes.low, &planes.high) else {
            return Ok(None);
        };
        let Some(uv) = lift_endpoints(planes.surfaces, planes.axis, &points, b)? else {
            return Ok(None);
        };
        let Some(excluded) = self.excluded_faces(&planes, &points[0]) else {
            return Ok(None);
        };
        Ok(Some(AxisMaterialChord {
            body: self.body(),
            axis: planes.axis,
            faces: planes.faces,
            uv,
            points,
            length: &planes.high - &planes.low,
            excluded_faces: excluded,
        }))
    }
    /// Candidate midpoint rays between opposite imported coordinate planes.
    /// Each accepted result has exact UV lift/membership, entry/exit normals
    /// and complete support-box exclusion of every other boundary face.
    /// Failure to find one is NOT proof that a smaller material chord is absent.
    pub fn find_axis_material_witness(
        &self,
        b: &mut Budget,
    ) -> Result<Option<AxisMaterialChord<'a>>> {
        let mut best: Option<AxisMaterialChord<'a>> = None;
        for pair in self.pairs() {
            let Some(witness) = self.pair_witness(pair.faces, b)? else {
                continue;
            };
            // The positive entering/exiting interval has no other boundary
            // intersection on the embedded outward-oriented admitted body.
            if self.complete() && witness.length_mm() < self.minimum_mm() {
                return Err(Error::new(
                    "BREP_ALIGNED_WALL_CONTRADICTION",
                    "A material witness contradicts the complete wall lower bound",
                ));
            }
            if (self.complete() && witness.length_mm() == self.minimum_mm())
                || (!self.complete() && witness.length_mm() < self.minimum_mm())
            {
                return Ok(Some(witness));
            }
            if best
                .as_ref()
                .is_none_or(|prior| witness.length_mm() < prior.length_mm())
            {
                best = Some(witness);
            }
        }
        Ok(best)
    }
    /// Exact minimum for THIS face-normal-aligned material-chord criterion.
    /// This is not a reach/medial-axis or arbitrary edge-normal-cone metric.
    pub fn exact_minimum_with(&self, witness: &AxisMaterialChord<'_>) -> Option<&Q> {
        (self.complete()
            && std::ptr::eq(self.body(), witness.body())
            && witness.length_mm() == self.minimum_mm())
        .then_some(self.minimum_mm())
    }
}
