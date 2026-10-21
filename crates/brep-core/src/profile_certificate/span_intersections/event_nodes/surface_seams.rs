//! Actual source-surface boundary identity for owned original UV seam pairs.
use super::*;
use nurbs_core::{
    periodic_trim::{SurfaceSeamProof, verify_surface_seam_identity},
    surface::Surface,
};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedOriginalSurfaceSeams {
    dictionary: Arc<OwnedOriginalSeamDictionary>,
    surface: Surface,
    proofs: [Option<SurfaceSeamProof>; 2],
}
fn derive(
    dictionary: &OwnedOriginalSeamDictionary,
    surface: &Surface,
) -> Result<[Option<SurfaceSeamProof>; 2]> {
    dictionary.reverify_source()?;
    surface.validate()?;
    let n = surface.control_points.len();
    let m = surface.control_points[0].len();
    let domain = [
        [surface.knots_u[surface.degree_u], surface.knots_u[n]],
        [surface.knots_v[surface.degree_v], surface.knots_v[m]],
    ];
    if domain != dictionary.chart().chart() {
        return Err(refused(
            "Original UV seam chart is not the actual supporting-surface domain",
        ));
    }
    let mut proofs = [None; 2];
    for axis in 0..2 {
        if dictionary.chart().wrapped()[axis] {
            proofs[axis] = Some(verify_surface_seam_identity(surface, axis)?.ok_or_else(|| {
                refused("Actual supporting-surface boundary identity remains unproven")
            })?);
        }
    }
    Ok(proofs)
}
impl OwnedOriginalSurfaceSeams {
    pub fn dictionary(&self) -> &Arc<OwnedOriginalSeamDictionary> {
        &self.dictionary
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn proofs(&self) -> &[Option<SurfaceSeamProof>; 2] {
        &self.proofs
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(&self.dictionary, &self.surface)? != self.proofs {
            return Err(refused("Original supporting-surface seam proof changed"));
        }
        Ok(())
    }
}
/// Bind all actual wrapped UV shores to the actual validated source surface.
/// Complete UV interval pairing plus exact authored-parameter surface boundary
/// identity proves full world-image equality under different carrier clocks.
/// No tolerance or fitted world curve is used. Generic trim chart coverage,
/// surface regularity/injectivity and quotient manifold face admission remain
/// independent obligations; this receipt does not construct a physical face.
pub fn bind_original_seams_to_surface(
    dictionary: Arc<OwnedOriginalSeamDictionary>,
    surface: &Surface,
) -> Result<OwnedOriginalSurfaceSeams> {
    let proofs = derive(&dictionary, surface)?;
    Ok(OwnedOriginalSurfaceSeams {
        dictionary,
        surface: surface.clone(),
        proofs,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_surface_seam_replay_rejects_changed_boundary_and_cached_kind() {
        let c = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&c, [0., 0., 1.]).unwrap();
        let v = [[0., 0.], [4., 0.], [4., 1.], [0., 1.], [0., 0.]];
        let wire = v
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let regions = Arc::new(
            assemble_original_filled_regions(
                &[wire],
                &[1],
                1e-12,
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                regions.owner().cycles().graph().clone(),
                [[0., 4.], [0., 1.]],
                [true, false],
                Default::default(),
            )
            .unwrap(),
        );
        let dictionary =
            Arc::new(own_original_seam_dictionary(regions, chart, Default::default()).unwrap());
        let s = bind_original_seams_to_surface(dictionary, &surface).unwrap();
        s.reverify_source().unwrap();
        let mut bad = s.clone();
        bad.proofs[0] = None;
        assert!(bad.reverify_source().is_err());
        let mut bad = s;
        let n = bad.surface.control_points.len();
        bad.surface.control_points[n - 1][0][0] = 0.01;
        assert!(bad.reverify_source().is_err());
    }
}
