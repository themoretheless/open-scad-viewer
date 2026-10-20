//! Original-coefficient curve/surface roots with strict interval Krawczyk proofs.
//! Boundary roots, tangencies and exhausted cells remain explicitly unresolved.
use crate::{
    Result, check, curve::Curve, distance_bounds::Interval as I, resource, surface::Surface,
};
type Vector = [I; 3];
type Box3 = [[f64; 2]; 3];
#[derive(Debug)]
pub struct Root {
    pub spans: [usize; 3],
    /// Exact dyadic subcell of the three original knot spans, owning one root.
    pub normalized_cell: Box3,
    /// Outward natural t/u/v enclosure of that unique root.
    pub parameter_box: Box3,
    pub position_bounds: [[f64; 2]; 3],
    /// Sign of C_t dot (S_u cross S_v) at this certified transverse root.
    pub crossing_sign: i8,
}
#[derive(Debug)]
pub struct Unresolved {
    pub spans: [usize; 3],
    pub normalized_cell: Box3,
    pub parameter_box: Box3,
}
#[derive(Debug)]
pub struct Report {
    pub roots: Vec<Root>,
    pub unresolved: Vec<Unresolved>,
    pub cells: usize,
    pub excluded: usize,
    /// Complete original parameter-product coverage, not a solid/trim proof.
    pub complete: bool,
}
#[derive(Clone)]
struct Net {
    dims: [usize; 3],
    data: Vec<Vector>,
}
impl Net {
    fn index(&self, p: [usize; 3]) -> usize {
        (p[0] * self.dims[1] + p[1]) * self.dims[2] + p[2]
    }
    fn point(&self, p: [usize; 3]) -> Vector {
        self.data[self.index(p)]
    }
    fn coordinates(&self) -> impl Iterator<Item = [usize; 3]> + '_ {
        (0..self.dims[0]).flat_map(move |i| {
            (0..self.dims[1]).flat_map(move |j| (0..self.dims[2]).map(move |k| [i, j, k]))
        })
    }
    fn split(&self, axis: usize, fraction: I) -> Result<[Self; 2]> {
        let complement = I::point(1.).sub(fraction)?.intersect(0., 1.)?;
        let mut out = [self.clone(), self.clone()];
        for base in self.coordinates().filter(|p| p[axis] == 0) {
            let mut q: Vec<_> = (0..self.dims[axis])
                .map(|i| {
                    let mut p = base;
                    p[axis] = i;
                    self.point(p)
                })
                .collect();
            let n = q.len();
            let mut left = q.clone();
            let mut right = q.clone();
            for r in 1..n {
                for i in 0..n - r {
                    for k in 0..3 {
                        let a = q[i][k];
                        let b = q[i + 1][k];
                        q[i][k] = a
                            .mul(complement)?
                            .add(b.mul(fraction)?)?
                            .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
                    }
                }
                left[r] = q[0];
                right[n - 1 - r] = q[n - 1 - r];
            }
            for side in 0..2 {
                for i in 0..n {
                    let mut p = base;
                    p[axis] = i;
                    let at = out[side].index(p);
                    out[side].data[at] = if side == 0 { left[i] } else { right[i] };
                }
            }
        }
        Ok(out)
    }
    fn center(&self) -> Result<Vector> {
        let mut current = self.clone();
        for axis in 0..3 {
            let mut dims = current.dims;
            dims[axis] = 1;
            let mut next = Net {
                dims,
                data: vec![[I::point(0.); 3]; dims.iter().product()],
            };
            for base in next.coordinates().collect::<Vec<_>>() {
                let mut q: Vec<_> = (0..current.dims[axis])
                    .map(|i| {
                        let mut p = base;
                        p[axis] = i;
                        current.point(p)
                    })
                    .collect();
                for r in 1..q.len() {
                    for i in 0..q.len() - r {
                        for k in 0..3 {
                            q[i][k] = q[i][k].add(q[i + 1][k])?.mul(I::point(0.5))?;
                        }
                    }
                }
                let at = next.index(base);
                next.data[at] = q[0];
            }
            current = next;
        }
        Ok(current.data[0])
    }
}
fn hull(values: impl Iterator<Item = I>) -> I {
    values.fold(
        I {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        },
        |a, b| I {
            lo: a.lo.min(b.lo),
            hi: a.hi.max(b.hi),
        },
    )
}
fn determinant(a: [[I; 3]; 3]) -> Result<I> {
    let mut sum = I::point(0.);
    for i in 0..3 {
        sum = sum.add(
            a[0][i].mul(
                a[1][(i + 1) % 3]
                    .mul(a[2][(i + 2) % 3])?
                    .sub(a[1][(i + 2) % 3].mul(a[2][(i + 1) % 3])?)?,
            )?,
        )?;
    }
    Ok(sum)
}
enum Verdict {
    Excluded,
    Unique([I; 3], i8),
    Unresolved,
}
fn krawczyk(net: &Net) -> Result<Verdict> {
    if (0..3).any(|k| {
        let h = hull(net.data.iter().map(|p| p[k]));
        h.lo > 0. || h.hi < 0.
    }) {
        return Ok(Verdict::Excluded);
    }
    let mut jac = [[I::point(0.); 3]; 3];
    for axis in 0..3 {
        let degree = net.dims[axis] - 1;
        if degree == 0 {
            return Ok(Verdict::Unresolved);
        }
        for k in 0..3 {
            let values = net
                .coordinates()
                .filter(|p| p[axis] < degree)
                .map(|p| {
                    let mut q = p;
                    q[axis] += 1;
                    net.point(q)[k]
                        .sub(net.point(p)[k])?
                        .mul(I::point(degree as f64))
                })
                .collect::<Result<Vec<_>>>()?;
            jac[k][axis] = hull(values.into_iter());
        }
    }
    let a = jac.map(|r| r.map(|x| x.lo * 0.5 + x.hi * 0.5));
    let det = (0..3)
        .map(|i| {
            a[0][i]
                * (a[1][(i + 1) % 3] * a[2][(i + 2) % 3] - a[1][(i + 2) % 3] * a[2][(i + 1) % 3])
        })
        .sum::<f64>();
    if !det.is_finite() || det == 0. {
        return Ok(Verdict::Unresolved);
    }
    let mut inverse = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            inverse[i][j] = (a[(j + 1) % 3][(i + 1) % 3] * a[(j + 2) % 3][(i + 2) % 3]
                - a[(j + 1) % 3][(i + 2) % 3] * a[(j + 2) % 3][(i + 1) % 3])
                / det;
        }
    }
    if !inverse.iter().flatten().all(|x| x.is_finite()) {
        return Ok(Verdict::Unresolved);
    }
    let d = determinant(inverse.map(|r| r.map(I::point)))?;
    if d.lo <= 0. && d.hi >= 0. {
        return Ok(Verdict::Unresolved);
    }
    let center = net.center()?;
    let mut image = [I::point(0.5); 3];
    let mut contraction = 0_f64;
    for i in 0..3 {
        for j in 0..3 {
            image[i] = image[i].sub(I::point(inverse[i][j]).mul(center[j])?)?;
        }
        let mut norm = I::point(0.);
        for k in 0..3 {
            let mut m = I::point(if i == k { 1. } else { 0. });
            for j in 0..3 {
                m = m.sub(I::point(inverse[i][j]).mul(jac[j][k])?)?;
            }
            norm = norm.add(I::point(m.lo.abs().max(m.hi.abs())))?;
            image[i] = image[i].add(m.mul(I::new(-0.5, 0.5)?)?)?;
        }
        contraction = contraction.max(norm.hi);
    }
    if image.iter().any(|x| x.hi < 0. || x.lo > 1.) {
        return Ok(Verdict::Excluded);
    }
    if contraction < 0.5 && image.iter().all(|x| x.lo > 0. && x.hi < 1.) {
        // ||I-RJ|| < 1 connects RJ to I through invertible matrices,
        // so det(RJ)>0. With the proved sign of det(R), this proves
        // the root orientation without sampling a normal or derivative.
        Ok(Verdict::Unique(image, if d.lo > 0. { 1 } else { -1 }))
    } else {
        Ok(Verdict::Unresolved)
    }
}
fn field_net(c: &Curve, s: &Surface, spans: [usize; 3], domain: Box3) -> Result<Net> {
    let ch = crate::curve_distance::restricted_controls(
        c,
        spans[0],
        I::new(domain[0][0], domain[0][1])?,
    )?;
    let sh = crate::curve_surface_composition::surface_net_on(
        s,
        [spans[1], spans[2]],
        [domain[1], domain[2]],
    )?;
    let origin = &c.control_points[spans[0] - c.degree];
    let dims = [ch.len(), sh.len(), sh[0].len()];
    let mut data = Vec::with_capacity(dims.iter().product());
    for cp in &ch {
        for row in &sh {
            for sp in row {
                let mut f = [I::point(0.); 3];
                for k in 0..3 {
                    // Curve extraction is translated by its original first span pole.
                    let hc = cp[k].add(I::point(origin[k]).mul(cp[3])?)?;
                    f[k] = hc.mul(sp[3])?.sub(sp[k].mul(cp[3])?)?;
                }
                data.push(f);
            }
        }
    }
    Ok(Net { dims, data })
}
struct Cell {
    net: Net,
    normalized: Box3,
    depth: usize,
}
fn map_box(span: Box3, cell: Box3, local: [I; 3]) -> Result<Box3> {
    let mut out = [[0.; 2]; 3];
    for k in 0..3 {
        let q = I::point(cell[k][0]).add(
            I::point(cell[k][1])
                .sub(I::point(cell[k][0]))?
                .mul(local[k])?,
        )?;
        let x = I::point(span[k][0])
            .add(I::point(span[k][1]).sub(I::point(span[k][0]))?.mul(q)?)?
            .intersect(span[k][0], span[k][1])?;
        out[k] = [x.lo, x.hi];
    }
    Ok(out)
}
// Avoid an ill-conditioned reparameterization when one coordinate is already
// at its binary64 enclosure floor while another still needs refinement. This
// enlarges a *search box*, never geometry or the root enclosure returned below.
// Every new inclusion must pass the unchanged strict Krawczyk conditions.
fn refinement_box(domain: Box3, original: Box3) -> Box3 {
    std::array::from_fn(|k| {
        let magnitude = original[k][0]
            .abs()
            .max(original[k][1].abs())
            .max(original[k][1] - original[k][0]);
        let floor = (magnitude * (256. * f64::EPSILON)).max(f64::from_bits(256));
        if domain[k][1] - domain[k][0] >= floor {
            return domain[k];
        }
        let middle = domain[k][0] * 0.5 + domain[k][1] * 0.5;
        [
            (middle - floor * 0.5)
                .next_down()
                .min(domain[k][0])
                .max(original[k][0]),
            (middle + floor * 0.5)
                .next_up()
                .max(domain[k][1])
                .min(original[k][1]),
        ]
    })
}

/// Complete parameter-product search on nonperiodic positive-weight NURBS.
/// Every returned root has a strict Krawczyk inclusion and contraction proof on
/// its original-span cell. Coefficients enclose the original homogeneous data;
/// approximate extracted patches and sampled residuals never authorize roots.
/// Root boxes may be broad: callers must independently establish positional
/// accuracy before authoring a vertex. No trim, uniqueness across distinct cells,
/// self-intersection, topology mutation or solid admission is implied.
pub fn intersections(c: &Curve, s: &Surface, max_cells: usize) -> Result<Report> {
    c.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3
            && s.control_points[0][0].len() == 3
            && !c.periodic
            && !s.periodic_u
            && !s.periodic_v,
        "Certified curve/surface roots need nonperiodic 3D sources",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Certified root work must be in 1..100000 cells",
    )?;
    if !((c.degree + 1)
        .checked_mul(s.degree_u + 1)
        .and_then(|n| n.checked_mul(s.degree_v + 1))
        .is_some_and(|n| n <= 4096))
    {
        return Err(resource(
            "Certified curve/surface degree product exceeds 4096",
        ));
    }
    let cs: Vec<_> = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect();
    let us: Vec<_> = (s.degree_u..s.control_points.len())
        .filter(|&i| s.knots_u[i] < s.knots_u[i + 1])
        .collect();
    let vs: Vec<_> = (s.degree_v..s.control_points[0].len())
        .filter(|&i| s.knots_v[i] < s.knots_v[i + 1])
        .collect();
    if !(cs
        .len()
        .checked_mul(us.len())
        .and_then(|n| n.checked_mul(vs.len()))
        .is_some_and(|n| n <= 1024))
    {
        return Err(resource(
            "Certified curve/surface span product exceeds 1024",
        ));
    }
    let mut out = Report {
        roots: vec![],
        unresolved: vec![],
        cells: 0,
        excluded: 0,
        complete: false,
    };
    for &ct in &cs {
        for &su in &us {
            for &sv in &vs {
                let ids = [ct, su, sv];
                let spans = [
                    [c.knots[ct], c.knots[ct + 1]],
                    [s.knots_u[su], s.knots_u[su + 1]],
                    [s.knots_v[sv], s.knots_v[sv + 1]],
                ];
                if out.cells == max_cells {
                    out.unresolved.push(Unresolved {
                        spans: ids,
                        normalized_cell: [[0., 1.]; 3],
                        parameter_box: spans,
                    });
                    continue;
                }
                let mut stack = vec![Cell {
                    net: field_net(c, s, ids, spans)?,
                    normalized: [[0., 1.]; 3],
                    depth: 0,
                }];
                while let Some(cell) = stack.pop() {
                    let unknown = |cell: &Cell| -> Result<Unresolved> {
                        Ok(Unresolved {
                            spans: ids,
                            normalized_cell: cell.normalized,
                            parameter_box: map_box(spans, cell.normalized, [I::new(0., 1.)?; 3])?,
                        })
                    };
                    if out.cells == max_cells {
                        out.unresolved.push(unknown(&cell)?);
                        for pending in stack.drain(..) {
                            out.unresolved.push(unknown(&pending)?);
                        }
                        break;
                    }
                    out.cells += 1;
                    match krawczyk(&cell.net)? {
                        Verdict::Excluded => {
                            out.excluded += 1;
                        }
                        Verdict::Unique(root, crossing_sign) => {
                            let mut domain = map_box(spans, cell.normalized, root)?;
                            // Existence/uniqueness is owned by the original cell.
                            // Rebuild refinements from original coefficients and
                            // retain that proof if the precision budget is exhausted.
                            for _ in 0..8 {
                                if out.cells == max_cells || domain.iter().any(|d| d[0] == d[1]) {
                                    break;
                                }
                                out.cells += 1;
                                let search = refinement_box(domain, spans);
                                match krawczyk(&field_net(c, s, ids, search)?)? {
                                    Verdict::Unique(next, refined_sign) => {
                                        if refined_sign != crossing_sign {
                                            return Err(crate::Error::new(
                                                "NURBS_ROOT_PROOF_CONTRADICTION",
                                                "Root orientation changed during certified refinement",
                                            ));
                                        }
                                        let refined = map_box(search, [[0., 1.]; 3], next)?;
                                        if refined == domain {
                                            break;
                                        }
                                        domain = refined;
                                    }
                                    Verdict::Unresolved => break,
                                    Verdict::Excluded => {
                                        return Err(crate::Error::new(
                                            "NURBS_ROOT_PROOF_CONTRADICTION",
                                            "Root refinement excluded an already certified root",
                                        ));
                                    }
                                }
                            }
                            let position = crate::curve_distance::enclosure(
                                c,
                                ct,
                                I::new(domain[0][0], domain[0][1])?,
                            )?;
                            out.roots.push(Root {
                                spans: ids,
                                normalized_cell: cell.normalized,
                                parameter_box: domain,
                                crossing_sign,
                                position_bounds: std::array::from_fn(|k| {
                                    [position[k].lo, position[k].hi]
                                }),
                            });
                        }
                        Verdict::Unresolved => {
                            if cell.depth == 48 {
                                out.unresolved.push(unknown(&cell)?);
                                continue;
                            }
                            let axis = (0..3)
                                .max_by(|&a, &b| {
                                    (cell.normalized[a][1] - cell.normalized[a][0])
                                        .total_cmp(&(cell.normalized[b][1] - cell.normalized[b][0]))
                                })
                                .unwrap();
                            // Bias the search cut away from common centered CAD
                            // roots. The stored binary64 cut is authoritative;
                            // enclose its actual ratio so rounded nonuniform
                            // subdivision cannot change the root's parameter map.
                            let a = cell.normalized[axis][0];
                            let b = cell.normalized[axis][1];
                            let middle = a + (b - a) * 0.375;
                            if !(a < middle && middle < b) {
                                out.unresolved.push(unknown(&cell)?);
                                continue;
                            }
                            let ratio = I::point(middle)
                                .sub(I::point(a))?
                                .div(I::point(b).sub(I::point(a))?)?
                                .intersect(0., 1.)?;
                            let [left, right] = cell.net.split(axis, ratio)?;
                            for (side, net) in [(1, right), (0, left)] {
                                let mut normalized = cell.normalized;
                                normalized[axis][1 - side] = middle;
                                stack.push(Cell {
                                    net,
                                    normalized,
                                    depth: cell.depth + 1,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    out.complete = out.unresolved.is_empty();
    out.roots.sort_by(|a, b| {
        a.parameter_box[0][0]
            .total_cmp(&b.parameter_box[0][0])
            .then(a.parameter_box[1][0].total_cmp(&b.parameter_box[1][0]))
            .then(a.parameter_box[2][0].total_cmp(&b.parameter_box[2][0]))
    });
    Ok(out)
}
