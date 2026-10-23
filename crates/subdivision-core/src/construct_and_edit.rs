//! Control cage construction (`from_faces`, `loft`, `extrude`, `sweep`, `revolve`),
//! inverse-limit fitting (`fit`), and freeform deformation / sculpting (`deform`, `brush`, `sculpt`).

use crate::{Cage, Point, Result, error};

impl Cage {
    pub fn from_faces(vertices: Vec<Point>, faces: Vec<Vec<usize>>) -> Result<Self> {
        let cage = Self { vertices, faces };
        cage.validate()?;
        Ok(cage)
    }
}

#[derive(Clone, Debug)]
pub struct Fit {
    pub cage: Cage,
    pub iterations: usize,
    pub vertex_residual_before_mm: f64,
    pub vertex_residual_after_mm: f64,
    pub correspondence: &'static str,
}

/// Fits cage positions so one Catmull–Clark step interpolates the original
/// vertices more closely. Not a unique inverse limit surface.
pub fn fit(cage: &Cage, iterations: usize) -> Result<Fit> {
    if iterations > 32 {
        return Err(error("Subdivision fitting requires 0..32 iterations"));
    }
    cage.validate()?;
    let mut cage = cage.clone();
    let target = cage.vertices.clone();
    // Candidates share the input's faces, so topology stays valid; only the
    // numeric coordinate budget can break. Check it cheaply per candidate.
    let within_budget = |c: &Cage| {
        c.vertices
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1e6)
    };
    let residual = |c: &Cage| -> Result<(f64, Vec<Point>)> {
        let r = c.subdivide_validated(1)?;
        let delta: Vec<Point> = target
            .iter()
            .zip(&r.cage.vertices)
            .map(|(a, b)| std::array::from_fn(|k| a[k] - b[k]))
            .collect();
        let error =
            (delta.iter().flatten().map(|x| x * x).sum::<f64>() / target.len() as f64).sqrt();
        Ok((error, delta))
    };
    let (before, mut delta) = residual(&cage)?;
    let mut after = before;
    let mut completed = 0;
    for _ in 0..iterations {
        if after <= 1e-12 {
            break;
        }
        let mut accepted = false;
        for step in [1., 0.5, 0.25, 0.125, 0.0625] {
            let mut candidate = cage.clone();
            for (p, d) in candidate.vertices.iter_mut().zip(&delta) {
                for k in 0..3 {
                    p[k] += step * d[k];
                }
            }
            if !within_budget(&candidate) {
                continue;
            }
            let (error, next) = residual(&candidate)?;
            if error < after {
                cage = candidate;
                after = error;
                delta = next;
                accepted = true;
                completed += 1;
                break;
            }
        }
        if !accepted {
            break;
        }
    }
    Ok(Fit {
        cage,
        iterations: completed,
        vertex_residual_before_mm: before,
        vertex_residual_after_mm: after,
        correspondence: "source_triangle_topology_one_refinement_step",
    })
}

impl Cage {
    pub fn deform(&self, operation: &geometry_ops::Deformation) -> Result<Self> {
        self.validate()?;
        operation.validate()?;
        let mut cage = self.clone();
        cage.vertices = self
            .vertices
            .iter()
            .map(|&p| operation.apply(p))
            .collect::<Result<_>>()?;
        cage.validate()?;
        Ok(cage)
    }
    pub fn brush(&self, brush: &geometry_ops::Brush) -> Result<Self> {
        self.validate()?;
        brush.validate()?;
        let mut cage = self.clone();
        cage.vertices = self
            .vertices
            .iter()
            .map(|&p| brush.apply(p))
            .collect::<Result<_>>()?;
        cage.validate()?;
        Ok(cage)
    }
    /// Newell face normals accumulated per control vertex, plus edge adjacency.
    pub fn sculpt_target(&self) -> Result<geometry_ops::SculptTarget> {
        self.validate()?;
        let faces = self.faces.iter().map(Vec::as_slice);
        geometry_ops::SculptTarget::from_faces(self.vertices.clone(), faces)
    }
    /// Sculpts control vertices; faces are unchanged and the cage stays valid.
    pub fn sculpt(&self, brush: &geometry_ops::SculptBrush) -> Result<Self> {
        let mut cage = self.clone();
        cage.vertices = self.sculpt_target()?.sculpt(brush)?;
        cage.validate()?;
        Ok(cage)
    }
}

impl Cage {
    /// Constructs a quad-sided cage directly; section start indices establish correspondence.
    pub fn loft(sections: &[Vec<Point>], caps: bool) -> Result<Self> {
        if !(2..=64).contains(&sections.len())
            || !(3..=64).contains(&sections[0].len())
            || sections.iter().any(|s| s.len() != sections[0].len())
        {
            return Err(error("Invalid subdivision loft sections"));
        }
        let n = sections[0].len();
        let vertices = sections.iter().flatten().copied().collect();
        let mut faces =
            Vec::with_capacity((sections.len() - 1) * n + if caps { 2 } else { 0 });
        for k in 0..sections.len() - 1 {
            for i in 0..n {
                faces.push(vec![
                    k * n + i,
                    k * n + (i + 1) % n,
                    (k + 1) * n + (i + 1) % n,
                    (k + 1) * n + i,
                ]);
            }
        }
        if caps {
            faces.push((0..n).rev().collect());
            faces.push(((sections.len() - 1) * n..sections.len() * n).collect());
        }
        let cage = Self { vertices, faces };
        cage.validate()?;
        Ok(cage)
    }
    pub fn extrude(profile: &[Point], vector: Point) -> Result<Self> {
        if vector.iter().any(|v| !v.is_finite())
            || vector.iter().map(|v| v * v).sum::<f64>() <= 1e-24
        {
            return Err(error("Invalid subdivision extrusion vector"));
        }
        Self::loft(
            &[
                profile.to_vec(),
                profile
                    .iter()
                    .map(|p| std::array::from_fn(|k| p[k] + vector[k]))
                    .collect(),
            ],
            true,
        )
    }
    pub fn sweep(profile: &[[f64; 2]], path: &[Point], up: Point, caps: bool) -> Result<Self> {
        Self::loft(&geometry_ops::sweep_sections(profile, path, up)?, caps)
    }
    pub fn revolve(profile: &[[f64; 2]], segments: usize) -> Result<Self> {
        if !(3..=64).contains(&segments)
            || !(3..=64).contains(&profile.len())
            || profile
                .iter()
                .any(|p| !p[0].is_finite() || !p[1].is_finite() || p[0] <= 0.)
        {
            return Err(error(
                "Subdivision revolve requires 3..64 segments and positive-radius profile",
            ));
        }
        let n = profile.len();
        let mut vertices = Vec::with_capacity(segments * n);
        for k in 0..segments {
            let a = 2. * std::f64::consts::PI * k as f64 / segments as f64;
            let (s, c) = a.sin_cos();
            vertices.extend(profile.iter().map(|p| [p[0] * c, p[0] * s, p[1]]));
        }
        let mut faces = Vec::with_capacity(segments * n);
        for k in 0..segments {
            for i in 0..n {
                faces.push(vec![
                    k * n + i,
                    k * n + (i + 1) % n,
                    ((k + 1) % segments) * n + (i + 1) % n,
                    ((k + 1) % segments) * n + i,
                ]);
            }
        }
        let cage = Self { vertices, faces };
        cage.validate()?;
        Ok(cage)
    }
}

