//! Actual original restriction + nonlinear candidate image identities, followed
//! by independent complete candidate nonzero-fill ownership.
use super::*;
mod faces;
pub use faces::{OwnedClockMappedFaces, assemble_clock_mapped_surface_faces};
use periodic::{ClockAliasSpec, ClockMappedSourceChart};
#[derive(Clone, Debug)]
pub struct ClockMappedPeriodicRegions {
    source: ClockMappedSourceChart,
    cycle_edges: Vec<Vec<usize>>,
    original_cycles: Vec<Vec<usize>>,
    regions: WindingRegularization,
}
fn graph(source: &ClockMappedSourceChart) -> Result<&WeightedChartGraph> {
    let g = source
        .graph()
        .ok_or_else(|| refused("Original clock-mapped chart remains unproven"))?;
    if !g.edges.is_empty() && !source.contact_ownership().is_some_and(|c| c.complete) {
        return Err(refused("Clock-mapped endpoint ownership remains unproven"));
    }
    Ok(g)
}
impl ClockMappedPeriodicRegions {
    pub fn source(&self) -> &ClockMappedSourceChart {
        &self.source
    }
    pub fn regions(&self) -> &WindingRegularization {
        &self.regions
    }
    pub fn cycle_edges(&self) -> &[Vec<usize>] {
        &self.cycle_edges
    }
    pub fn original_cycles(&self) -> &[Vec<usize>] {
        &self.original_cycles
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.source.reverify()?;
        let g = graph(&self.source)?;
        let c = compress(g)?;
        if self.cycle_edges != c.edges
            || self.original_cycles != c.original_cycles
            || self.regions.source() != materialize(g, &c.edges)
            || self.regions.coefficients() != c.coefficients
        {
            return Err(refused("Original nonlinear clock region binding changed"));
        }
        self.regions.verify()
    }
}
/// Own original winding through exact actual curve-clock image compaction and
/// full candidate profile/nonzero-fill certification. General distinct-wire
/// contacts and actual supporting-surface face ownership remain separate; no
/// strict-source isotopy receipt is fabricated for coincident original images.
pub fn regularize_clock_mapped_periodic_winding(
    sources: &[Curve],
    partitions: &[Report],
    windings: &[[i32; 2]],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    specs: &[ClockAliasSpec],
    tolerance: f64,
    max_work: u64,
) -> Result<ClockMappedPeriodicRegions> {
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(refused(
            "Clock mapped regions need positive finite tolerance",
        ));
    }
    let source = periodic::assemble_clock_mapped_source_chart(
        sources, partitions, windings, chart, anchor, limits, specs, max_work,
    )?;
    let g = graph(&source)?;
    let c = compress(g)?;
    let regions =
        regularize_nonzero_winding(&materialize(g, &c.edges), &c.coefficients, tolerance)?;
    let owner = ClockMappedPeriodicRegions {
        source,
        cycle_edges: c.edges,
        original_cycles: c.original_cycles,
        regions,
    };
    owner.reverify_source()?;
    Ok(owner)
}

/// Original nonlinear image regions bound to an owned actual support. This is
/// a correspondence owner; injectivity, lifts and sewn face admission are separate.
#[derive(Clone, Debug)]
pub struct ClockMappedSurfaceRegions {
    surface: crate::Surface,
    regions: ClockMappedPeriodicRegions,
    wrapped: [bool; 2],
    limits: periodic::SurfaceCorrespondenceLimits,
}
impl ClockMappedSurfaceRegions {
    pub fn surface(&self) -> &crate::Surface {
        &self.surface
    }
    pub fn regions(&self) -> &ClockMappedPeriodicRegions {
        &self.regions
    }
    /// Fresh report; a stale cached acceptance flag is never published.
    pub fn correspondence(&self) -> Result<periodic::SurfaceCorrespondence> {
        self.regions.reverify_source()?;
        clock_surface_correspondence(&self.surface, &self.regions, self.wrapped, self.limits)
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.correspondence().map(|_| ())
    }
}
fn clock_surface_correspondence(
    surface: &crate::Surface,
    regions: &ClockMappedPeriodicRegions,
    wrapped: [bool; 2],
    limits: periodic::SurfaceCorrespondenceLimits,
) -> Result<periodic::SurfaceCorrespondence> {
    let source = regions.source.source();
    let report = source.verify_surface(surface, wrapped, limits)?;
    if !report.within_world_tolerance {
        return Err(refused(
            "Original clock surface deviation or seam remains unproven",
        ));
    }
    let chart = source.chart();
    let g = graph(&regions.source)?;
    for component in regions.regions.components() {
        for &wire in component.source_loops() {
            for &edge in &regions.cycle_edges[wire] {
                let e = &g.edges[edge];
                if e.contributions.iter().any(|p| p.source.is_none())
                    && (0..2).any(|axis| {
                        !wrapped[axis]
                            && (0..2).any(|side| {
                                e.curve
                                    .control_points
                                    .iter()
                                    .all(|p| p[axis] == chart[axis][side])
                            })
                    })
                {
                    return Err(refused(
                        "Retained clock connector uses a nonwrapped support axis",
                    ));
                }
            }
        }
    }
    Ok(report)
}
/// Bind original exact restrictions and mapped image regions to an actual
/// surface. No source isotopy or physical face certificate is manufactured.
pub fn bind_clock_mapped_regions_to_surface(
    surface: &crate::Surface,
    regions: &ClockMappedPeriodicRegions,
    wrapped: [bool; 2],
    limits: periodic::SurfaceCorrespondenceLimits,
) -> Result<ClockMappedSurfaceRegions> {
    regions.reverify_source()?;
    clock_surface_correspondence(surface, regions, wrapped, limits)?;
    Ok(ClockMappedSurfaceRegions {
        surface: surface.clone(),
        regions: regions.clone(),
        wrapped,
        limits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_support_and_original_region_bindings_are_freshly_verified() {
        let curve = Curve::from_polyline(vec![
            vec![1., 1.],
            vec![2., 1.],
            vec![2., 2.],
            vec![1., 2.],
            vec![1., 1.],
        ])
        .unwrap();
        let part = Report {
            pieces: vec![periodic::Piece {
                source_domain: curve.domain(),
                curve: curve.clone(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        };
        let regions = regularize_clock_mapped_periodic_winding(
            &[curve],
            &[part],
            &[[0; 2]],
            [[0., 8.]; 2],
            [1.5; 2],
            ChartAssemblyLimits {
                max_segments: 64,
                max_links: 32,
                max_edges: 32,
                max_images: 256,
                max_cells: 30000,
                tolerance_uv: 1e-8,
            },
            &[],
            1e-7,
            1_000_000,
        )
        .unwrap();
        let surface = crate::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 8., 8.],
            knots_v: vec![0., 0., 8., 8.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 8., 0.]],
                vec![vec![8., 0., 0.], vec![8., 8., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let owner = bind_clock_mapped_regions_to_surface(
            &surface,
            &regions,
            [false; 2],
            periodic::SurfaceCorrespondenceLimits {
                tolerance_uv: 1e-6,
                tolerance_world: 1e-4,
                max_rectangles: 256,
                max_deviation_cells: 100000,
            },
        )
        .unwrap();
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.surface.knots_u = vec![0., 0., 7., 7.];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.wrapped[0] = true;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.regions.cycle_edges.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.regions.original_cycles.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.limits.tolerance_world = f64::NAN;
        assert!(bad.reverify_source().is_err());
    }
}
