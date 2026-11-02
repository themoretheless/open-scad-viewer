//! Axis-aligned box CSG via coordinate-cell classification, not general mesh CSG.
use crate::{Result, math::Bounds, mesh::Mesh, require};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boolean {
    Union,
    Intersection,
    Difference,
    Xor,
}
/// Box-set boundary with planar triangles. Geometric self-contact may yield a
/// nonmanifold boundary; that result is refused instead of welded arbitrarily.
pub fn boxes(a: &[Bounds], b: &[Bounds], operation: Boolean) -> Result<Mesh> {
    let cells = box_cells(a, b, operation)?;
    mesh_cells(&cells)
}
/// Occupied coordinate cells, enabling subsequent box-set operations without
/// recovering boxes from tessellated geometry. Budget includes all inputs.
pub fn box_cells(a: &[Bounds], b: &[Bounds], operation: Boolean) -> Result<Vec<Bounds>> {
    require(a.len() + b.len() <= 64, "box-set budget is 64")?;
    // Bounds fields are public: revalidate before coordinate classification.
    for v in a.iter().chain(b) {
        Bounds::new(v.min, v.max)?;
    }
    if a.is_empty() && b.is_empty() {
        return Ok(vec![]);
    }
    let axes: [Vec<f64>; 3] = std::array::from_fn(|i| {
        let mut x = a
            .iter()
            .chain(b)
            .flat_map(|v| [v.min[i], v.max[i]])
            .collect::<Vec<_>>();
        x.sort_by(f64::total_cmp);
        x.dedup();
        x
    });
    let dims = axes.each_ref().map(|a| a.len() - 1);
    let [nx, ny, nz] = dims;
    require(
        nx * ny * nz <= 262144,
        "box coordinate cell budget exceeded",
    )?;
    let index = |i: usize, j: usize, k: usize| (i * ny + j) * nz + k;
    let mut occupied = vec![false; nx * ny * nz];
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                // Classify interval inclusion, not floating midpoints: adjacent binary64
                // endpoints may have no representable interior midpoint.
                let inside = |set: &[Bounds]| {
                    set.iter().any(|v| {
                        (0..3).all(|d| {
                            let q = [i, j, k][d];
                            axes[d][q] >= v.min[d] && axes[d][q + 1] <= v.max[d]
                        })
                    })
                };
                let x = inside(a);
                let y = inside(b);
                occupied[index(i, j, k)] = match operation {
                    Boolean::Union => x || y,
                    Boolean::Intersection => x && y,
                    Boolean::Difference => x && !y,
                    Boolean::Xor => x != y,
                };
            }
        }
    }
    let mut cells = Vec::new();
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                if occupied[index(i, j, k)] {
                    let q = [i, j, k];
                    cells.push(Bounds::new(
                        std::array::from_fn(|d| axes[d][q[d]]),
                        std::array::from_fn(|d| axes[d][q[d] + 1]),
                    )?);
                }
            }
        }
    }
    // Keep future box-set operations within the same explicit operand budget.
    require(cells.len() <= 64, "box result exceeds reusable cell budget")?;
    Ok(cells)
}
pub fn mesh_cells(cells: &[Bounds]) -> Result<Mesh> {
    require(cells.len() <= 64, "box-set budget is 64")?;
    for b in cells {
        Bounds::new(b.min, b.max)?;
    }
    if cells.is_empty() {
        return Ok(Mesh::empty());
    }
    let axes: [Vec<f64>; 3] = std::array::from_fn(|i| {
        let mut x = cells
            .iter()
            .flat_map(|v| [v.min[i], v.max[i]])
            .collect::<Vec<_>>();
        x.sort_by(f64::total_cmp);
        x.dedup();
        x
    });
    let dims = axes.each_ref().map(|a| a.len() - 1);
    let [nx, ny, nz] = dims;
    require(
        nx * ny * nz <= 262144,
        "box coordinate cell budget exceeded",
    )?;
    let index = |i: usize, j: usize, k: usize| (i * ny + j) * nz + k;
    let mut occupied = vec![false; nx * ny * nz];
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let q = [i, j, k];
                occupied[index(i, j, k)] = cells.iter().any(|v| {
                    (0..3).all(|d| axes[d][q[d]] >= v.min[d] && axes[d][q[d] + 1] <= v.max[d])
                });
            }
        }
    }
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    let mut ids = BTreeMap::<[usize; 3], usize>::new();
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                if !occupied[index(i, j, k)] {
                    continue;
                }
                let cell = [i, j, k];
                for axis in 0..3 {
                    for side in 0..2 {
                        let q = cell[axis];
                        let neighbor = if side == 0 {
                            q.checked_sub(1)
                        } else if q + 1 < dims[axis] {
                            Some(q + 1)
                        } else {
                            None
                        };
                        if neighbor.is_some_and(|q| {
                            let mut c = cell;
                            c[axis] = q;
                            occupied[index(c[0], c[1], c[2])]
                        }) {
                            continue;
                        }
                        let u = (axis + 1) % 3;
                        let v = (axis + 2) % 3;
                        let mut corners = [[0; 3]; 4];
                        for (n, [du, dv]) in
                            [[0, 0], [1, 0], [1, 1], [0, 1]].into_iter().enumerate()
                        {
                            corners[n] = cell;
                            corners[n][axis] += side;
                            corners[n][u] += du;
                            corners[n][v] += dv;
                        }
                        let face = corners.map(|c| {
                            *ids.entry(c).or_insert_with(|| {
                                let id = vertices.len();
                                vertices.push(std::array::from_fn(|d| axes[d][c[d]]));
                                id
                            })
                        });
                        let mut f = [[face[0], face[1], face[2]], [face[0], face[2], face[3]]];
                        if side == 0 {
                            for t in &mut f {
                                t.swap(1, 2)
                            }
                        }
                        triangles.extend(f);
                    }
                }
            }
        }
    }
    let result = Mesh::new(vertices, triangles)?;
    require(
        result.topology().is_closed_manifold(),
        "box result has nonmanifold contact",
    )?;
    Ok(result)
}
/// Inward shell of one axis-aligned box. Optional opening is (axis, high_side).
pub fn shell_box(
    outer: Bounds,
    thickness: f64,
    opening: Option<(usize, bool)>,
) -> Result<Vec<Bounds>> {
    Bounds::new(outer.min, outer.max)?;
    require(
        thickness.is_finite()
            && thickness > 0.
            && (0..3).all(|i| thickness < (outer.max[i] - outer.min[i]) / 2.),
        "positive shell thickness below half each extent required",
    )?;
    let mut lo = outer.min.map(|x| x + thickness);
    let mut hi = outer.max.map(|x| x - thickness);
    if let Some((axis, high)) = opening {
        require(axis < 3, "opening axis must be 0..2")?;
        if high {
            hi[axis] = outer.max[axis]
        } else {
            lo[axis] = outer.min[axis]
        }
    }
    box_cells(&[outer], &[Bounds::new(lo, hi)?], Boolean::Difference)
}
