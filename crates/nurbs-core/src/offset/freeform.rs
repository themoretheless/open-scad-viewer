//! Retained freeform normal offset S+d*n and whole-region local regularity.
//! The exact-real offset remains an expression over the immutable source;
//! a fitted NURBS approximation is a separate, explicitly sampled result.
use crate::{
    Error, Result,
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
    surface_shape_operator,
};
mod fitting;

fn refused(message: &str) -> Error {
    Error::new("NURBS_FREEFORM_OFFSET_REFUSED", message)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
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
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
#[derive(Clone, Debug)]
pub struct NormalOffset {
    source: Surface,
    distance: f64,
}
#[derive(Clone, Debug)]
pub struct Evaluation {
    pub point: [f64; 3],
    pub tangent_u: [f64; 3],
    pub tangent_v: [f64; 3],
    pub normal: [f64; 3],
}
/// Private construction binds the whole-region local regularity proof to the
/// actual source and distance. Global injectivity/self-trimming are separate.
pub struct RegularRegion {
    offset: NormalOffset,
    domain: [[f64; 2]; 2],
    minimum_stretch: f64,
    cells: usize,
}
impl RegularRegion {
    pub fn offset(&self) -> &NormalOffset {
        &self.offset
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        self.domain
    }
    pub fn minimum_stretch(&self) -> f64 {
        self.minimum_stretch
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn evaluate(&self, uv: [f64; 2]) -> Result<Evaluation> {
        if (0..2)
            .any(|k| !uv[k].is_finite() || uv[k] < self.domain[k][0] || uv[k] > self.domain[k][1])
        {
            return Err(refused("Query outside the qualified offset region"));
        }
        self.offset.evaluate(uv)
    }
}
impl NormalOffset {
    pub fn new(source: &Surface, distance: f64) -> Result<Self> {
        source.validate()?;
        if !distance.is_finite()
            || source.control_points.iter().flatten().any(|p| p.len() != 3)
            || source.periodic_u
            || source.periodic_v
        {
            return Err(refused(
                "Freeform offset needs finite distance and a nonperiodic 3D chart",
            ));
        }
        Ok(Self {
            source: source.clone(),
            distance,
        })
    }
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn distance(&self) -> f64 {
        self.distance
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        let s = &self.source;
        [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ]
    }
    /// Analytic first derivatives; no finite-difference normals or cage offset.
    pub fn evaluate(&self, uv: [f64; 2]) -> Result<Evaluation> {
        let e = self.source.evaluate(uv[0], uv[1])?;
        let (u, v) = e
            .first_derivatives()
            .ok_or_else(|| refused("Source first derivatives unresolved"))?;
        let (uu, uvv, vv) = e
            .second_derivatives()
            .ok_or_else(|| refused("Source second derivatives unresolved"))?;
        let area = cross(u, v);
        let length = norm(area);
        if !length.is_finite() || length <= 0. {
            return Err(refused("Singular source normal"));
        }
        let n = area.map(|x| x / length);
        let normal_derivative = |dn: [f64; 3]| -> [f64; 3] {
            let radial = dot(n, dn);
            std::array::from_fn(|i| (dn[i] - n[i] * radial) / length)
        };
        let nu = normal_derivative(add(cross(uu, v), cross(u, uvv)));
        let nv = normal_derivative(add(cross(uvv, v), cross(u, vv)));
        let tangent_u = std::array::from_fn(|i| u[i] + self.distance * nu[i]);
        let tangent_v = std::array::from_fn(|i| v[i] + self.distance * nv[i]);
        let area = cross(tangent_u, tangent_v);
        let length = norm(area);
        if !length.is_finite() || length <= 0. {
            return Err(refused("Offset has a focal singularity at this parameter"));
        }
        let point = std::array::from_fn(|i| e.point[i] + self.distance * n[i]);
        if point
            .iter()
            .chain(&tangent_u)
            .chain(&tangent_v)
            .any(|x| !x.is_finite())
        {
            return Err(refused("Offset evaluation overflow"));
        }
        Ok(Evaluation {
            point,
            tangent_u,
            tangent_v,
            normal: area.map(|x| x / length),
        })
    }
    /// Sufficient whole-region bound: ||d*shape||_2 < 1. Every source tensor
    /// interval is outward bounded; unresolved rectangles subdivide under one
    /// shared work budget. Success proves local regularity, not global embedding.
    pub fn qualify(&self, domain: [[f64; 2]; 2], max_cells: usize) -> Result<RegularRegion> {
        let natural = self.domain();
        if !(1..=100000).contains(&max_cells)
            || (0..2).any(|k| {
                domain[k].iter().any(|v| !v.is_finite())
                    || domain[k][0] < natural[k][0]
                    || domain[k][0] >= domain[k][1]
                    || domain[k][1] > natural[k][1]
            })
        {
            return Err(refused("Invalid offset region or shared proof budget"));
        }
        let mut queue = vec![(domain, 0usize)];
        let mut cells = 0;
        let mut minimum = f64::INFINITY;
        while let Some((query, depth)) = queue.pop() {
            if cells == max_cells {
                return Err(refused("Offset regularity proof budget exhausted"));
            }
            let report = surface_shape_operator::rectangle_bounds(
                &self.source,
                query,
                (max_cells - cells).min(64),
            )?;
            cells += report.cells.max(1);
            if let Some(tensor) = report.shape.tensor {
                let tensor = tensor
                    .into_iter()
                    .flatten()
                    .map(|x| I::new(x[0], x[1]))
                    .collect::<Result<Vec<_>>>()?;
                let upper = box_distance(&tensor, &vec![I::point(0.); 9])?.1;
                let stretch = I::point(1.)
                    .sub(I::point(self.distance.abs()).mul(I::new(0., upper)?)?)?
                    .lo;
                if stretch > 0. {
                    minimum = minimum.min(stretch);
                    continue;
                }
            }
            if depth == 16 || cells >= max_cells {
                return Err(refused("Offset local regularity remains unproven"));
            }
            let axis = usize::from(
                (query[1][1] - query[1][0]) / (natural[1][1] - natural[1][0])
                    > (query[0][1] - query[0][0]) / (natural[0][1] - natural[0][0]),
            );
            let middle = query[axis][0] / 2. + query[axis][1] / 2.;
            if middle <= query[axis][0] || middle >= query[axis][1] {
                return Err(refused("Offset proof precision exhausted"));
            }
            let mut a = query;
            let mut b = query;
            a[axis][1] = middle;
            b[axis][0] = middle;
            queue.push((b, depth + 1));
            queue.push((a, depth + 1));
        }
        Ok(RegularRegion {
            offset: self.clone(),
            domain,
            minimum_stretch: minimum,
            cells,
        })
    }
    /// Existing NURBS fitting remains a sampled proposal, even when recognition
    /// heuristics detect an analytic family. Never promote it to an exact proof.
    pub fn approximate(&self, tolerance: f64) -> Result<super::surface_offset::OffsetReport> {
        fitting::approximate(self, tolerance)
    }
}
