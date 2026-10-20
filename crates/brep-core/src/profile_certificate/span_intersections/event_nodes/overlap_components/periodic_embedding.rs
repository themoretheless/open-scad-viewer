//! Full actual supporting-surface rank, local cover and cross-chart embedding.
use super::*;
#[derive(Clone, Debug)]
pub struct OwnedOriginalPeriodicEmbedding {
    wires: Arc<OwnedOriginalPeriodicWires>,
    max_regularity_cells: usize,
    atlas_limits: OriginalInjectiveAtlasLimits,
    pair_limits: OriginalAtlasPairLimits,
    regularity: OriginalSurfaceRegularityRecord,
    charts: Vec<OriginalInjectiveChart>,
    created_cells: usize,
    atlas_span_work: usize,
    bounds: Vec<[[f64; 2]; 3]>,
    pairs: Vec<OriginalAtlasPair>,
    pair_span_work: usize,
    subdivision_cells: usize,
}
type Data = (
    OriginalSurfaceRegularityRecord,
    Vec<OriginalInjectiveChart>,
    usize,
    usize,
    Vec<[[f64; 2]; 3]>,
    Vec<OriginalAtlasPair>,
    usize,
    usize,
);
fn derive(
    wires: &OwnedOriginalPeriodicWires,
    max_cells: usize,
    atlas: OriginalInjectiveAtlasLimits,
    pairs: OriginalAtlasPairLimits,
) -> Result<Data> {
    wires.reverify_source()?;
    let supporting = wires.surface();
    let surface = supporting.surface();
    let report = nurbs_core::surface_regularity::inspect(surface, max_cells)?;
    if !report.spanwise_regular
        || !report.unresolved.is_empty()
        || report.cells == 0
        || report.proven_cells == 0
    {
        return Err(refused(
            "Actual periodic supporting surface has unproved nonzero one-sided normals",
        ));
    }
    let regularity = OriginalSurfaceRegularityRecord {
        cells: report.cells,
        proven_cells: report.proven_cells,
        interior_basis_c1: report.interior_basis_c1,
    };
    let (charts, created, work) = super::super::injective_atlas::cover_surface(surface, atlas)?;
    let (bounds, proofs, pair_work, subdivision_cells) =
        super::super::atlas_pairs::cover_source_pairs(
            surface,
            &charts,
            supporting.pieces().chart(),
            supporting.pieces().wrapped(),
            pairs,
        )?;
    if proofs.iter().any(|p| !p.proof.is_proven()) {
        return Err(refused(
            "Actual periodic supporting-surface cross-chart embedding has unresolved obligations",
        ));
    }
    Ok((
        regularity,
        charts,
        created,
        work,
        bounds,
        proofs,
        pair_work,
        subdivision_cells,
    ))
}
impl OwnedOriginalPeriodicEmbedding {
    pub fn wires(&self) -> &Arc<OwnedOriginalPeriodicWires> {
        &self.wires
    }
    pub fn regularity(&self) -> &OriginalSurfaceRegularityRecord {
        &self.regularity
    }
    pub fn charts(&self) -> &[OriginalInjectiveChart] {
        &self.charts
    }
    pub fn pairs(&self) -> &[OriginalAtlasPair] {
        &self.pairs
    }
    pub fn bounds(&self) -> &[[[f64; 2]; 3]] {
        &self.bounds
    }
    pub fn created_cells(&self) -> usize {
        self.created_cells
    }
    pub fn atlas_span_work(&self) -> usize {
        self.atlas_span_work
    }
    pub fn pair_span_work(&self) -> usize {
        self.pair_span_work
    }
    pub fn subdivision_cells(&self) -> usize {
        self.subdivision_cells
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (r, c, n, w, b, p, pw, sc) = derive(
            &self.wires,
            self.max_regularity_cells,
            self.atlas_limits,
            self.pair_limits,
        )?;
        if r != self.regularity
            || c != self.charts
            || n != self.created_cells
            || w != self.atlas_span_work
            || b != self.bounds
            || p != self.pairs
            || pw != self.pair_span_work
            || sc != self.subdivision_cells
        {
            return Err(refused(
                "Actual periodic source embedding proof or work census changed",
            ));
        }
        Ok(())
    }
}
/// Strong whole-source gate: every knot rectangle closure and every unordered
/// chart pair must pass. Unresolved, degenerate and multiply covered supports
/// never become regular/embedded by samples. Trim-local singular exclusions,
/// inter-wire contact topology and manifold face publication remain separate.
pub fn own_original_periodic_embedding(
    wires: Arc<OwnedOriginalPeriodicWires>,
    max_regularity_cells: usize,
    atlas_limits: OriginalInjectiveAtlasLimits,
    pair_limits: OriginalAtlasPairLimits,
) -> Result<OwnedOriginalPeriodicEmbedding> {
    let (
        regularity,
        charts,
        created_cells,
        atlas_span_work,
        bounds,
        pairs,
        pair_span_work,
        subdivision_cells,
    ) = derive(&wires, max_regularity_cells, atlas_limits, pair_limits)?;
    Ok(OwnedOriginalPeriodicEmbedding {
        wires,
        max_regularity_cells,
        atlas_limits,
        pair_limits,
        regularity,
        charts,
        created_cells,
        atlas_span_work,
        bounds,
        pairs,
        pair_span_work,
        subdivision_cells,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_regular_cover_pair_and_work_tampering() {
        let c = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&c, [0., 0., 1.]).unwrap();
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![4., 0.5]]).unwrap();
        let pieces = Arc::new(
            own_original_periodic_pieces(
                &[a],
                [[0., 4.], [0., 1.]],
                [true, false],
                1e-12,
                ParameterOrderLimits::default(),
                32,
            )
            .unwrap(),
        );
        let support = Arc::new(
            bind_original_periodic_pieces_to_surface(
                pieces,
                &surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let wires = Arc::new(
            own_original_periodic_wires(
                support,
                &[vec![OriginalPeriodicSourceUse {
                    source: 0,
                    reversed: false,
                }]],
            )
            .unwrap(),
        );
        let x = own_original_periodic_embedding(
            wires,
            4096,
            OriginalInjectiveAtlasLimits::default(),
            OriginalAtlasPairLimits::default(),
        )
        .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.regularity.proven_cells += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.charts.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.pairs.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.pair_span_work += 1;
        assert!(bad.reverify_source().is_err());
    }
}
