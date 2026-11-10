//! Point membership for an admitted root solid. Complete original rays and
//! whole-root-domain UV boxes authorize parity; missing work stays unresolved.
use super::*;
use crate::ray_parity;
use nurbs_core::trim_domain::Location;
#[derive(Clone, Copy)]
pub struct RootSolidPointLimits {
    pub tolerance_uv: f64,
    pub ray_cells: usize,
    pub domain_queries: usize,
    pub arc_subdivisions: usize,
    pub band_queries: usize,
    pub winding_queries: usize,
    pub winding: ss::OriginalSsWindingLimits,
    pub geometry: ss::OriginalSsCurveLimits,
}
impl Default for RootSolidPointLimits {
    fn default() -> Self {
        Self {
            tolerance_uv: 1e-8,
            ray_cells: 100000,
            domain_queries: 8192,
            arc_subdivisions: 8,
            band_queries: 16384,
            winding_queries: 1024,
            winding: ss::OriginalSsWindingLimits {
                max_segments: 4096,
                ..Default::default()
            },
            geometry: Default::default(),
        }
    }
}
#[derive(Debug)]
pub struct RootSolidRayReport {
    pub parity: Option<bool>,
    pub crossings: Vec<ray_parity::Crossing>,
    pub unresolved: Vec<ray_parity::Unresolved>,
    pub cells: usize,
    pub domain_queries: usize,
}
#[derive(Debug)]
pub struct RootSolidPointReport {
    pub parity: Option<bool>,
    pub attempts: Vec<RootSolidRayReport>,
    pub cells: usize,
    pub domain_queries: usize,
    pub band_queries: usize,
    pub winding_queries: usize,
    pub seeds: usize,
    pub propagations: usize,
    pub reason: &'static str,
}
impl QualifiedPairedRootSolid {
    pub fn classify_point(
        &self,
        point: [f64; 3],
        directions: &[[f64; 3]],
        limits: RootSolidPointLimits,
    ) -> Result<RootSolidPointReport> {
        if !point.iter().all(|x| x.is_finite())
            || !(1..=16).contains(&directions.len())
            || directions
                .iter()
                .any(|d| !d.iter().all(|x| x.is_finite()) || d.iter().all(|x| *x == 0.))
            || !limits.tolerance_uv.is_finite()
            || limits.tolerance_uv <= 0.
            || !(1..=1000000).contains(&limits.ray_cells)
            || !(1..=65536).contains(&limits.domain_queries)
            || !(1..=64).contains(&limits.arc_subdivisions)
            || !limits.arc_subdivisions.is_power_of_two()
            || limits.band_queries > 1000000
            || limits.winding_queries > 65536
        {
            return Err(unproved("Invalid root-solid point query or work allowance"));
        }
        let domain_limits = RootBoundaryVolumeLimits {
            arc_subdivisions: limits.arc_subdivisions,
            band_queries: limits.band_queries,
            winding_queries: limits.winding_queries,
            winding: limits.winding,
            geometry: limits.geometry,
            ..Default::default()
        };
        // Only domain work counters are used; this does not perform or infer a volume.
        let mut work = RootBoundaryVolumeReport {
            signed_bounds: None,
            accuracy_proven: false,
            cells: 0,
            spans: 0,
            band_queries: 0,
            winding_queries: 0,
            seeds: 0,
            propagations: 0,
            reason: "point-domain-work",
        };
        let mut out = RootSolidPointReport {
            parity: None,
            attempts: Vec::new(),
            cells: 0,
            domain_queries: 0,
            band_queries: 0,
            winding_queries: 0,
            seeds: 0,
            propagations: 0,
            reason: "root-band-work-unproved",
        };
        let Some(mut domains) = domain::Domains::new(self.boundary(), domain_limits, &mut work)?
        else {
            return Ok(out);
        };
        let surfaces: Vec<_> = self
            .model()
            .topology()
            .faces
            .iter()
            .map(|f| &f.surface)
            .collect();
        for (i, &direction) in directions.iter().enumerate() {
            let cells = limits.ray_cells - out.cells;
            let queries = limits.domain_queries - out.domain_queries;
            if cells == 0 || queries == 0 {
                break;
            }
            let remaining = directions.len() - i;
            let attempt = ray_parity::classify_surface_ray(
                &surfaces,
                point,
                direction,
                limits.tolerance_uv,
                (cells / remaining).max(1),
                (queries / remaining).max(1),
                |face, uv, _| {
                    let mut classification =
                        domains.classify(face, uv, domain_limits, &mut work)?;
                    if classification.is_none() {
                        classification =
                            domains.classify_box(face, uv, domain_limits, &mut work)?;
                    }
                    let location = match classification {
                        Some(true) => Location::Inside,
                        Some(false) => Location::Outside,
                        None => Location::Unresolved,
                    };
                    Ok((location, 1))
                },
            )?;
            out.cells += attempt.cells;
            out.domain_queries += attempt.domain_cells;
            out.parity = attempt.parity;
            out.attempts.push(RootSolidRayReport {
                parity: attempt.parity,
                crossings: attempt.crossings,
                unresolved: attempt.unresolved,
                cells: attempt.cells,
                domain_queries: attempt.domain_cells,
            });
            if out.parity.is_some() {
                break;
            }
        }
        out.band_queries = work.band_queries;
        out.winding_queries = work.winding_queries;
        out.seeds = work.seeds;
        out.propagations = work.propagations;
        out.reason = if out.parity.is_some() {
            "complete-root-trimmed-point-parity"
        } else {
            "root-trimmed-point-unresolved"
        };
        Ok(out)
    }
}
