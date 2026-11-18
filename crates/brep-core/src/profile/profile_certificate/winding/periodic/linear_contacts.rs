//! Original source-bound periodic line-image contacts and nonzero arrangement.
use super::*;
use crate::profile_certificate::{LinearWindingArrangement, regularize_linear_winding_arrangement};
#[derive(Clone, Debug)]
pub struct LinearPeriodicContactRegions {
    original: periodic::ExactSourceChart,
    regions: LinearWindingArrangement,
}
fn loops(source: &periodic::ExactSourceChart) -> Result<Vec<Vec<Curve>>> {
    let g = source
        .materialized_graph()
        .ok_or_else(|| refused("Missing corrected original periodic graph"))?;
    Ok(g.loops
        .iter()
        .map(|cycle| cycle.iter().map(|&i| g.edges[i].curve.clone()).collect())
        .collect())
}
impl LinearPeriodicContactRegions {
    pub fn original(&self) -> &periodic::ExactSourceChart {
        &self.original
    }
    pub fn regions(&self) -> &LinearWindingArrangement {
        &self.regions
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.original.reverify()?;
        let source = loops(&self.original)?;
        if self.regions.source() != source || self.regions.coefficients() != vec![1; source.len()] {
            return Err(refused(
                "Periodic contact arrangement is detached from its corrected original graph",
            ));
        }
        self.regions.reverify_source()
    }
}
/// Complete exact line-image contact regularization on the ORIGINAL corrected
/// periodic chart. Crossings, overlap, opposite cancellation, self-crossings and
/// endpoint fans are arranged before nonzero selection. All partition sources
/// and perimeter correction remain owned/replayed; no endpoint-only report is
/// mistaken for a complete contact census. Curved carriers and physical surface
/// faces for this new arrangement proof kind remain separate.
pub fn regularize_linear_periodic_contacts(
    sources: &[Curve],
    partitions: &[Report],
    windings: &[[i32; 2]],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    max_work: u64,
) -> Result<LinearPeriodicContactRegions> {
    let original = periodic::assemble_exact_source_chart(
        sources, partitions, windings, chart, anchor, limits, max_work,
    )?;
    original.reverify()?;
    let source = loops(&original)?;
    let regions = regularize_linear_winding_arrangement(&source, &vec![1; source.len()])?;
    let owner = LinearPeriodicContactRegions { original, regions };
    owner.reverify_source()?;
    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periodic_linear_contact_regions_cannot_inherit_unrelated_arrangement_or_weights() {
        let c = Curve::from_polyline(vec![
            vec![0.125, 0.125],
            vec![0.625, 0.125],
            vec![0.625, 0.625],
            vec![0.125, 0.625],
            vec![0.125, 0.125],
        ])
        .unwrap();
        let part = Report {
            pieces: vec![periodic::Piece {
                curve: c.clone(),
                source_domain: c.domain(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        };
        let limits = ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 64,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        };
        let owner = regularize_linear_periodic_contacts(
            &[c.clone()],
            &[part],
            &[[0; 2]],
            [[0., 1.]; 2],
            [0.25, 0.25],
            limits,
            1_000_000,
        )
        .unwrap();
        let mut bad = owner.clone();
        bad.regions = regularize_linear_winding_arrangement(owner.regions.source(), &[2]).unwrap();
        assert!(bad.reverify_source().is_err());
        let other = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ])
        .unwrap();
        let mut bad = owner;
        bad.regions = regularize_linear_winding_arrangement(&[vec![other]], &[1]).unwrap();
        assert!(bad.reverify_source().is_err());
    }
}
