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
impl<'a> AlignedWallCoverage<'a> {
    /// Candidate midpoint rays between opposite imported coordinate planes.
    /// Each accepted result has exact UV lift/membership, entry/exit normals
    /// and complete support-box exclusion of every other boundary face.
    /// Failure to find one is NOT proof that a smaller material chord is absent.
    pub fn find_axis_material_witness(
        &self,
        b: &mut Budget,
    ) -> Result<Option<AxisMaterialChord<'a>>> {
        let body = self.body();
        let top = body.topology();
        let mut best: Option<AxisMaterialChord<'a>> = None;
        for pair in self.pairs() {
            let [i, j] = pair.faces;
            if i == j {
                continue;
            }
            let (FaceGeometry::Imported(a), FaceGeometry::Imported(z)) =
                (&top.faces[i].surface, &top.faces[j].surface)
            else {
                continue;
            };
            let (Some(na), Some(nz)) = (self.frames[i].normal(), self.frames[j].normal()) else {
                continue;
            };
            let (Some((axis, sa)), Some((other, sz))) = (exact_axis(na), exact_axis(nz)) else {
                continue;
            };
            if axis != other || sa == sz {
                continue;
            }
            let ba = self.frames[i].bounds();
            let bz = self.frames[j].bounds();
            if ba[axis][0] != ba[axis][1] || bz[axis][0] != bz[axis][1] {
                continue;
            }
            let (low, high, faces, surfaces, signs) = if ba[axis][0] < bz[axis][0] {
                (
                    ba[axis][0].clone(),
                    bz[axis][0].clone(),
                    [i, j],
                    [a, z],
                    [sa, sz],
                )
            } else {
                (
                    bz[axis][0].clone(),
                    ba[axis][0].clone(),
                    [j, i],
                    [z, a],
                    [sz, sa],
                )
            };
            if low >= high || signs != [-1, 1] {
                continue;
            }
            let mut points: [[_; 3]; 2] =
                std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
            let mut overlap = true;
            for k in 0..3 {
                if k == axis {
                    points[0][k] = low.clone();
                    points[1][k] = high.clone();
                } else {
                    let lo = ba[k][0].clone().max(bz[k][0].clone());
                    let hi = ba[k][1].clone().min(bz[k][1].clone());
                    if lo >= hi {
                        overlap = false;
                        break;
                    }
                    let value = (lo + hi) / Q::from_integer(2.into());
                    points[0][k] = value.clone();
                    points[1][k] = value;
                }
            }
            if !overlap {
                continue;
            }
            let mut uv: [[Q; 2]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
            let mut admitted = true;
            for end in 0..2 {
                let Some(p) =
                    polynomial_plane_point(surfaces[end].source(), axis, points[end].clone(), b)?
                else {
                    admitted = false;
                    break;
                };
                let lines = surfaces[end]
                    .uses
                    .iter()
                    .map(|u| u.uv.clone())
                    .collect::<Vec<_>>();
                if !convex_uv_strict_contains(&lines, &p, b)? {
                    admitted = false;
                    break;
                }
                uv[end] = p;
            }
            if !admitted {
                continue;
            }
            let mut excluded = Vec::new();
            for k in 0..top.faces.len() {
                if faces.contains(&k) {
                    continue;
                }
                let bounds = self.frames[k].bounds();
                let off_axis = (0..3)
                    .filter(|&d| d != axis)
                    .any(|d| points[0][d] < bounds[d][0] || points[0][d] > bounds[d][1]);
                let off_segment = bounds[axis][1] < low || bounds[axis][0] > high;
                if off_axis || off_segment {
                    excluded.push(k);
                } else {
                    admitted = false;
                    break;
                }
            }
            if !admitted {
                continue;
            }
            let length = &high - &low;
            // The positive entering/exiting interval has no other boundary
            // intersection on the embedded outward-oriented admitted body.
            if self.complete() && length < *self.minimum_mm() {
                return Err(Error::new(
                    "BREP_ALIGNED_WALL_CONTRADICTION",
                    "A material witness contradicts the complete wall lower bound",
                ));
            }
            let witness = AxisMaterialChord {
                body,
                axis,
                faces,
                uv,
                points,
                length,
                excluded_faces: excluded,
            };
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
