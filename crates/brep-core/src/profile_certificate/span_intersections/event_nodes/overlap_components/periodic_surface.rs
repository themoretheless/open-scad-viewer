//! Actual source-surface admission for exact translated periodic trim views.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use arithmetic::branch_neighbourhood::ParameterValue;
use nurbs_core::{
    periodic_trim::{SurfaceSeamProof, verify_surface_seam_identity},
    surface::Surface,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicPieceCoverage {
    pub piece: usize,
    pub source_cell: usize,
    pub critical_points: [usize; 4],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalPeriodicSurface {
    pieces: Arc<OwnedOriginalPeriodicPieces>,
    surface: Surface,
    limits: ParameterOrderLimits,
    seam_proofs: [Option<SurfaceSeamProof>; 2],
    coverage: Vec<OriginalPeriodicPieceCoverage>,
}
fn derive(
    pieces: &OwnedOriginalPeriodicPieces,
    surface: &Surface,
    l: ParameterOrderLimits,
) -> Result<(
    [Option<SurfaceSeamProof>; 2],
    Vec<OriginalPeriodicPieceCoverage>,
)> {
    if l.max_refinements > 512 || !(1..=128).contains(&l.max_resultant_size) {
        return Err(refused(
            "Invalid original periodic surface admission limits",
        ));
    }
    pieces.reverify_source()?;
    surface.validate()?;
    let n = surface.control_points.len();
    let m = surface.control_points[0].len();
    let domain = [
        [surface.knots_u[surface.degree_u], surface.knots_u[n]],
        [surface.knots_v[surface.degree_v], surface.knots_v[m]],
    ];
    if pieces.chart() != domain {
        return Err(refused(
            "Original periodic trim chart is not the actual surface domain",
        ));
    }
    let chart = domain.map(|a| a.map(|q| Q::from_float(q).unwrap()));
    let mut seam_proofs = [None; 2];
    for axis in 0..2 {
        if pieces.wrapped()[axis] {
            seam_proofs[axis]=Some(verify_surface_seam_identity(surface,axis)?.ok_or_else(||refused("Actual supporting surface does not prove the advertised periodic boundary identity"))?);
        }
    }
    let original = pieces
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut coverage = Vec::new();
    for (piece, p) in pieces.pieces().iter().enumerate() {
        let c = &original[p.source][p.cell];
        let offsets = [
            decode_ratio(&p.translation[0])?,
            decode_ratio(&p.translation[1])?,
        ];
        // Translate only the exact homogeneous coefficients used by the proof.
        // The original Curve stays untouched; no f64 shifted control net exists.
        let net = c
            .net
            .iter()
            .map(|h| {
                [
                    &h[0] + &offsets[0] * &h[2],
                    &h[1] + &offsets[1] * &h[2],
                    h[2].clone(),
                ]
            })
            .collect::<Vec<_>>();
        let bound = |cut: usize| -> Result<ParameterValue> {
            let v = &pieces.cuts()[p.source][cut].value;
            Ok(ParameterValue {
                primitive: decode_polynomial(&v.primitive)?,
                value: decode_polynomial(&v.parameter)?,
                interval: [decode_ratio(&v.interval[0])?, decode_ratio(&v.interval[1])?],
            })
        };
        let receipt = arithmetic::chart_coverage::prove(
            &[(net, c.domain.clone())],
            &[bound(p.cuts[0])?, bound(p.cuts[1])?],
            &chart,
            l.max_refinements,
            l.max_resultant_size,
        )?;
        if receipt.len() != 1 || receipt[0].0 != 0 {
            return Err(refused(
                "Periodic surface admission omitted an original trim cell",
            ));
        }
        coverage.push(OriginalPeriodicPieceCoverage {
            piece,
            source_cell: p.cell,
            critical_points: receipt[0].1,
        });
    }
    // Every transition was proved as an exact whole-period UV difference.
    // Closed chart coverage forces nonzero differences onto opposite shores;
    // these actual surface boundary identities therefore prove equal world
    // positions, including two-axis corner transitions, without sampling S.
    Ok((seam_proofs, coverage))
}
impl OwnedOriginalPeriodicSurface {
    pub fn pieces(&self) -> &Arc<OwnedOriginalPeriodicPieces> {
        &self.pieces
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn seam_proofs(&self) -> &[Option<SurfaceSeamProof>; 2] {
        &self.seam_proofs
    }
    pub fn coverage(&self) -> &[OriginalPeriodicPieceCoverage] {
        &self.coverage
    }
    /// Every world trace is S(original_pcurve(t)+exact_translation), retaining
    /// original natural cuts and the actual source surface, never a fitted 3D curve.
    pub fn reverify_source(&self) -> Result<()> {
        let (s, c) = derive(&self.pieces, &self.surface, self.limits)?;
        if s != self.seam_proofs || c != self.coverage {
            return Err(refused(
                "Original periodic world-curve seam identity or chart coverage changed",
            ));
        }
        Ok(())
    }
}
/// Surface source/seam/domain admission only. This does not certify surface
/// regularity/injectivity or construct a manifold face/wire quotient.
pub fn bind_original_periodic_pieces_to_surface(
    pieces: Arc<OwnedOriginalPeriodicPieces>,
    surface: &Surface,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalPeriodicSurface> {
    let (seam_proofs, coverage) = derive(&pieces, surface, limits)?;
    Ok(OwnedOriginalPeriodicSurface {
        pieces,
        surface: surface.clone(),
        limits,
        seam_proofs,
        coverage,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_missing_actual_surface_seam_proof_and_coverage() {
        let c = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&c, [0., 0., 1.]).unwrap();
        let a = Curve::from_polyline(vec![vec![9., 0.2], vec![17., 0.8]]).unwrap();
        let p = Arc::new(
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
        let x =
            bind_original_periodic_pieces_to_surface(p, &surface, ParameterOrderLimits::default())
                .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.seam_proofs[0] = None;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.coverage.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.surface.control_points[0][0][0] += 0.25;
        assert!(bad.reverify_source().is_err());
    }
}
