//! Uniform material on a connected Cartesian enclosure, after complete
//! original root-trimmed boundary separation and a certified parity witness.
use super::*;
use nurbs_core::interval_eval::{evaluate_surface_interval, Interval};
use std::collections::VecDeque;
#[derive(Clone, Copy)]
pub struct RootMaterialBoxLimits {
    pub separation_cells: usize,
    pub domain_queries: usize,
    pub point: RootSolidPointLimits,
}
impl Default for RootMaterialBoxLimits {
    fn default() -> Self {
        Self {
            separation_cells: 32768,
            domain_queries: 4096,
            point: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RootMaterialBoxWork {
    pub separation_cells: usize,
    pub domain_queries: usize,
    pub band_queries: usize,
    pub winding_queries: usize,
}
pub struct QualifiedRootMaterialBox {
    solid: Arc<QualifiedPairedRootSolid>,
    bounds: [[f64; 2]; 3],
    witness: [f64; 3],
    inside: bool,
    work: RootMaterialBoxWork,
    parity: RootSolidPointReport,
    directions: Vec<[f64; 3]>,
    limits: RootMaterialBoxLimits,
}
impl QualifiedRootMaterialBox {
    pub fn solid(&self) -> &Arc<QualifiedPairedRootSolid> {
        &self.solid
    }
    pub fn bounds(&self) -> [[f64; 2]; 3] {
        self.bounds
    }
    pub fn witness(&self) -> [f64; 3] {
        self.witness
    }
    pub fn inside(&self) -> bool {
        self.inside
    }
    pub fn separation_work(&self) -> &RootMaterialBoxWork {
        &self.work
    }
    pub fn parity_proof(&self) -> &RootSolidPointReport {
        &self.parity
    }
    pub fn reverify(&self) -> Result<()> {
        self.solid.reverify()?;
        let fresh = self
            .solid
            .qualify_material_box(self.bounds, &self.directions, self.limits)?;
        if fresh.inside != self.inside || fresh.witness != self.witness || fresh.work != self.work {
            return Err(unproved("Root material enclosure proof changed"));
        }
        Ok(())
    }
}
impl QualifiedPairedRootSolid {
    /// Every point in the closed box has the same material parity. No sample
    /// authorizes this owner until the entire retained boundary is excluded.
    pub fn qualify_material_box(
        self: &Arc<Self>,
        bounds: [[f64; 2]; 3],
        directions: &[[f64; 3]],
        limits: RootMaterialBoxLimits,
    ) -> Result<QualifiedRootMaterialBox> {
        if bounds
            .iter()
            .any(|d| !d.iter().all(|x| x.is_finite()) || d[0] > d[1])
            || !(1..=1000000).contains(&limits.separation_cells)
            || !(1..=65536).contains(&limits.domain_queries)
            || !(1..=16).contains(&directions.len())
            || directions
                .iter()
                .any(|d| !d.iter().all(|x| x.is_finite()) || d.iter().all(|x| *x == 0.))
        {
            return Err(unproved(
                "Invalid root material enclosure query or work allowance",
            ));
        }
        let p = limits.point;
        if !(1..=64).contains(&p.arc_subdivisions)
            || !p.arc_subdivisions.is_power_of_two()
            || p.band_queries > 1000000
            || p.winding_queries > 65536
        {
            return Err(unproved("Invalid root material enclosure domain allowance"));
        }
        let dl = RootBoundaryVolumeLimits {
            arc_subdivisions: p.arc_subdivisions,
            band_queries: p.band_queries,
            winding_queries: p.winding_queries,
            winding: p.winding,
            geometry: p.geometry,
            ..Default::default()
        };
        let mut domain_work = RootBoundaryVolumeReport {
            signed_bounds: None,
            accuracy_proven: false,
            cells: 0,
            spans: 0,
            band_queries: 0,
            winding_queries: 0,
            seeds: 0,
            propagations: 0,
            reason: "material-box-domain-work",
        };
        let mut domains = domain::Domains::new(self.boundary(), dl, &mut domain_work)?
            .ok_or_else(|| unproved("Root material enclosure boundary bands incomplete"))?;
        let mut work = RootMaterialBoxWork {
            separation_cells: 0,
            domain_queries: 0,
            band_queries: 0,
            winding_queries: 0,
        };
        let mut pending = VecDeque::new();
        for (face, f) in self.model().topology().faces.iter().enumerate() {
            let s = &f.surface;
            // The qualified outer Jordan loop bounds the complete retained
            // domain; intersect ALL original knot spans with its full UV box.
            for u in s.degree_u..s.control_points.len() {
                for v in s.degree_v..s.control_points[0].len() {
                    let d = [
                        [
                            s.knots_u[u].max(domains.roots[face][0][0]),
                            s.knots_u[u + 1].min(domains.roots[face][0][1]),
                        ],
                        [
                            s.knots_v[v].max(domains.roots[face][1][0]),
                            s.knots_v[v + 1].min(domains.roots[face][1][1]),
                        ],
                    ];
                    if d.iter().any(|r| r[0] >= r[1]) {
                        continue;
                    }
                    if pending.len() == limits.separation_cells {
                        return Err(unproved(
                            "Root material enclosure initial span work exhausted",
                        ));
                    }
                    pending.push_back((face, d, 0usize));
                }
            }
        }
        if pending.is_empty() {
            return Err(unproved(
                "Root material enclosure has no complete source spans",
            ));
        }
        while let Some((face, d, depth)) = pending.pop_front() {
            if work.separation_cells == limits.separation_cells {
                return Err(unproved(
                    "Root material enclosure separation work exhausted",
                ));
            }
            work.separation_cells += 1;
            let image = evaluate_surface_interval(
                &self.model().topology().faces[face].surface,
                Interval::new(d[0][0], d[0][1])?,
                Interval::new(d[1][0], d[1][1])?,
            )?;
            if (0..3).any(|k| image[k].hi < bounds[k][0] || bounds[k][1] < image[k].lo) {
                continue;
            }
            if work.domain_queries < limits.domain_queries {
                work.domain_queries += 1;
                let mut location = domains.classify(face, d, dl, &mut domain_work)?;
                if location.is_none() {
                    location = domains.classify_box(face, d, dl, &mut domain_work)?;
                }
                if location == Some(false) {
                    continue;
                }
                if location == Some(true)
                    && (0..3).all(|k| bounds[k][0] <= image[k].lo && image[k].hi <= bounds[k][1])
                {
                    return Err(unproved(
                        "Root material enclosure contains a retained boundary patch",
                    ));
                }
            }
            let axis = usize::from(d[1][1] - d[1][0] > d[0][1] - d[0][0]);
            let m = d[axis][0] * 0.5 + d[axis][1] * 0.5;
            if depth == 64 || !(d[axis][0] < m && m < d[axis][1]) {
                return Err(unproved(
                    "Root material enclosure touches boundary or separation is unresolved",
                ));
            }
            let mut a = d;
            let mut b = d;
            a[axis][1] = m;
            b[axis][0] = m;
            pending.push_back((face, b, depth + 1));
            pending.push_back((face, a, depth + 1));
        }
        work.band_queries = domain_work.band_queries;
        work.winding_queries = domain_work.winding_queries;
        let witness = bounds.map(|d| (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1]));
        // Separation and witness parity have separate explicit allowances.
        let parity = self.classify_point(witness, directions, p)?;
        let inside = parity
            .parity
            .ok_or_else(|| unproved("Root material enclosure witness parity unresolved"))?;
        Ok(QualifiedRootMaterialBox {
            solid: self.clone(),
            bounds,
            witness,
            inside,
            work,
            parity,
            directions: directions.to_vec(),
            limits,
        })
    }
}
