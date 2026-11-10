//! Physical obligations shared with coherent regions, distinct original owner.
use super::super::face_build::{self, FaceBuild, FaceContext};
use super::*;
#[derive(Clone)]
pub struct OwnedClockMappedFaces {
    source: ClockMappedSurfaceRegions,
    limits: PeriodicFaceLimits,
    built: FaceBuild,
}
impl OwnedClockMappedFaces {
    pub fn source(&self) -> &ClockMappedSurfaceRegions {
        &self.source
    }
    pub fn model(&self) -> &crate::Model {
        &self.built.model
    }
    pub fn domains(&self) -> &[[[f64; 2]; 2]] {
        &self.built.domains
    }
    pub fn injectivity(&self) -> &[nurbs_core::surface_injectivity::Report] {
        &self.built.injectivity
    }
    pub fn seam_edges(&self) -> &[usize] {
        &self.built.seam_edges
    }
    pub fn agreement(&self) -> &crate::boundary_agreement::Report {
        &self.built.agreement
    }
    pub fn boundary_error_upper(&self) -> f64 {
        self.built.boundary_error_upper
    }
    pub fn global_spatial_embedding_certified(&self) -> bool {
        false
    }
    /// Rebuild from actual original curve-clock sources and actual support;
    /// compare the entire model and all physical proof diagnostics.
    pub fn reverify(&self) -> Result<()> {
        let fresh = assemble_clock_mapped_surface_faces(&self.source, self.limits)?;
        self.built.verify_against(&fresh.built)
    }
    pub fn verify_model(&self, model: &crate::Model) -> Result<()> {
        let fresh = assemble_clock_mapped_surface_faces(&self.source, self.limits)?;
        if fresh.built.model != *model {
            return Err(refused(
                "Saved clock mapped model differs from its original source ownership",
            ));
        }
        Ok(())
    }
}
/// Construct retained faces from the original exact restriction and nonlinear
/// image-clock proof kind. Full chart injectivity, continuous boundary lifts,
/// original connector sewing, vertex links and saved model agreement are shared
/// with coherent faces; no coherent-isotopy receipt is fabricated.
/// Cross-face spatial embedding and closed volume admission remain separate.
pub fn assemble_clock_mapped_surface_faces(
    source: &ClockMappedSurfaceRegions,
    limits: PeriodicFaceLimits,
) -> Result<OwnedClockMappedFaces> {
    let correspondence = source.correspondence()?;
    let regions = &source.regions;
    let built = face_build::build(
        FaceContext {
            surface: &source.surface,
            wrapped: source.wrapped,
            chart: regions.source.source().chart(),
            components: regions.regions.components(),
            cycle_edges: &regions.cycle_edges,
            graph: graph(&regions.source)?,
            error_upper: correspondence.error_upper,
        },
        limits,
    )?;
    Ok(OwnedClockMappedFaces {
        source: source.clone(),
        limits,
        built,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_clock_faces_replay_geometry_source_and_every_physical_receipt() {
        let c = Curve::from_polyline(vec![
            vec![1., 1.],
            vec![2., 1.],
            vec![2., 2.],
            vec![1., 2.],
            vec![1., 1.],
        ])
        .unwrap();
        let part = Report {
            pieces: c
                .split(2.)
                .unwrap()
                .into_iter()
                .map(|curve| periodic::Piece {
                    source_domain: curve.domain(),
                    curve,
                    translation: [0.; 2],
                    tile: [0; 2],
                })
                .collect(),
            candidate_hulls_contained: false,
        };
        let regions = regularize_clock_mapped_periodic_winding(
            &[c],
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
        let support = bind_clock_mapped_regions_to_surface(
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
        let owner =
            assemble_clock_mapped_surface_faces(&support, PeriodicFaceLimits::default()).unwrap();
        owner.reverify().unwrap();
        let mut bad = owner.clone();
        bad.built.injectivity[0].proven = false;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.built.injectivity[0].spans += 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.built.agreement.uses.clear();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.built.agreement.cells += 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.built.domains[0][0][1] += 1.;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.built.seam_edges.push(0);
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.built.boundary_error_upper = 0.;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.built.model.vertices[0].point[0] = bad.built.model.vertices[0].point[0].next_up();
        assert!(bad.reverify().is_err());
        let mut bad = owner;
        bad.source.regions.original_cycles.clear();
        assert!(bad.reverify().is_err());
    }
}
