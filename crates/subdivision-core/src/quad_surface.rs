//! Retained Catmull–Clark geometry on oriented manifold quad topology.
//! Regular queries resolve to exact bicubic patches, not tessellation triangles.
use crate::{BicubicPatch, Cage, Point, Result, edge, error};
pub mod editing;

#[derive(Clone, Debug)]
pub struct QuadSurface {
    cage: Cage,
}
#[derive(Clone, Debug)]
pub struct LocatedPatch {
    pub patch: BicubicPatch,
    pub uv: [f64; 2],
    /// Local patch coordinates differentiated with respect to original face UV.
    pub parameter_jacobian: [[f64; 2]; 2],
    pub refinement_depth: usize,
}
impl QuadSurface {
    pub fn new(cage: Cage) -> Result<Self> {
        cage.validate()?;
        if cage.faces.iter().any(|f| f.len() != 4) {
            return Err(error("Quad surface requires quad faces"));
        }
        Ok(Self { cage })
    }
    pub fn cage(&self) -> &Cage {
        &self.cage
    }
    pub fn with_vertex(&self, index: usize, point: Point) -> Result<Self> {
        let mut cage = self.cage.clone();
        *cage
            .vertices
            .get_mut(index)
            .ok_or_else(|| error("Unknown cage vertex"))? = point;
        Self::new(cage)
    }
    fn check(&self, face: usize, uv: [f64; 2], depth: usize) -> Result<()> {
        if face >= self.cage.faces.len()
            || uv.iter().any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
            || depth > 5
        {
            return Err(error(
                "Invalid quad surface face, parameters or refinement budget",
            ));
        }
        Ok(())
    }
    /// Returns a retained uniform bicubic patch wherever finite refinement
    /// isolates a regular face. At extraordinary points and unresolved queries
    /// this refuses: no finite NURBS patch is fabricated for the singular chart.
    pub fn locate(
        &self,
        mut face: usize,
        mut uv: [f64; 2],
        max_depth: usize,
    ) -> Result<LocatedPatch> {
        self.check(face, uv, max_depth)?;
        let mut cage = self.cage.clone();
        let mut jacobian = [[1., 0.], [0., 1.]];
        for depth in 0..=max_depth {
            if let Some(patch) = cage
                .to_nurbs_patches()?
                .into_iter()
                .find(|p| p.face_index == face)
            {
                return Ok(LocatedPatch {
                    patch,
                    uv,
                    parameter_jacobian: jacobian,
                    refinement_depth: depth,
                });
            }
            if depth == max_depth {
                break;
            }
            let (child, next, rotation) = child_coordinates(uv);
            // The existing owner emits four children in original corner order.
            cage = cage.subdivide_validated(1)?.cage;
            face = 4 * face + child;
            uv = next;
            jacobian = std::array::from_fn(|i| {
                std::array::from_fn(|j| (0..2).map(|k| rotation[i][k] * jacobian[k][j]).sum())
            });
        }
        Err(error(
            "Regular patch not isolated within subdivision budget",
        ))
    }
    /// Exact-real limit position (binary64 arithmetic). Vertices use the owning
    /// limit stencil; original boundary edges use their cubic limit curve.
    /// Interior parameters use retained regular patches after finite refinement.
    /// This is not a rounding-error certificate or a global embedding claim.
    pub fn evaluate(&self, face: usize, uv: [f64; 2], max_depth: usize) -> Result<Point> {
        self.check(face, uv, max_depth)?;
        let f = &self.cage.faces[face];
        let [u, v] = uv;
        if (u == 0. || u == 1.) && (v == 0. || v == 1.) {
            let corner = match (u == 1., v == 1.) {
                (false, false) => 0,
                (true, false) => 1,
                (true, true) => 2,
                (false, true) => 3,
            };
            return Ok(self.cage.evaluate_limit_points()?[f[corner]]);
        }
        let boundary = if v == 0. {
            Some((0, u))
        } else if u == 1. {
            Some((1, v))
        } else if v == 1. {
            Some((2, 1. - u))
        } else if u == 0. {
            Some((3, 1. - v))
        } else {
            None
        };
        if let Some((i, t)) = boundary {
            let topology = self.cage.build_edges();
            let a = f[i];
            let b = f[(i + 1) % 4];
            if topology[&edge(a, b)].len() == 1 {
                let neighbor = |vertex, exclude| -> Result<usize> {
                    topology
                        .iter()
                        .find_map(|(&(x, y), uses)| {
                            if uses.len() != 1 {
                                return None;
                            }
                            if x == vertex && y != exclude {
                                Some(y)
                            } else if y == vertex && x != exclude {
                                Some(x)
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| error("Incomplete boundary fan"))
                };
                let controls =
                    [neighbor(a, b)?, a, b, neighbor(b, a)?].map(|i| self.cage.vertices[i]);
                let weights = [
                    (1. - t).powi(3) / 6.,
                    (3. * t * t * t - 6. * t * t + 4.) / 6.,
                    (-3. * t * t * t + 3. * t * t + 3. * t + 1.) / 6.,
                    t * t * t / 6.,
                ];
                return Ok(std::array::from_fn(|k| {
                    (0..4).map(|i| weights[i] * controls[i][k]).sum()
                }));
            }
        }
        let located = self.locate(face, uv, max_depth)?;
        Ok(located.patch.evaluate(located.uv[0], located.uv[1]))
    }
}
fn child_coordinates([u, v]: [f64; 2]) -> (usize, [f64; 2], [[f64; 2]; 2]) {
    match (u <= 0.5, v <= 0.5) {
        (true, true) => (0, [2. * u, 2. * v], [[2., 0.], [0., 2.]]),
        (false, true) => (1, [2. * v, 2. * (1. - u)], [[0., 2.], [-2., 0.]]),
        (false, false) => (2, [2. * (1. - u), 2. * (1. - v)], [[-2., 0.], [0., -2.]]),
        (true, false) => (3, [2. * (1. - v), 2. * u], [[0., -2.], [2., 0.]]),
    }
}
