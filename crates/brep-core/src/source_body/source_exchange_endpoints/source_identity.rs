//! Same original Body provenance without serialization or coordinate proximity.
use super::*;
use crate::{source_contour_proposal::SourceRegion, source_shell_incidence::Address};
pub(super) struct BodySource<E> {
    regions: Vec<SourceRegion>,
    edges: Vec<E>,
    uses: Vec<[Address; 2]>,
    poles: Vec<(Address, [f64; 3])>,
    inverse: Option<([usize; 2], f64)>,
}
impl<E: ExchangeCarrier> BodySource<E> {
    pub fn capture(body: &crate::source_volume::Body<E>) -> Result<Self> {
        let shell = body.geometry().shell();
        Ok(Self {
            regions: shell
                .regions()
                .ok_or_else(|| invalid("Qualified source material regions required"))?
                .to_vec(),
            edges: shell.edges().to_vec(),
            uses: shell.uses().to_vec(),
            poles: shell.poles().iter().map(|(a, p)| (*a, p.point())).collect(),
            inverse: body
                .geometry()
                .inverse_shear()
                .map(|p| (p.axes(), p.coefficient())),
        })
    }
    pub fn matches(&self, body: &crate::source_volume::Body<E>) -> bool {
        let shell = body.geometry().shell();
        self.inverse
            == body
                .geometry()
                .inverse_shear()
                .map(|p| (p.axes(), p.coefficient()))
            && self.uses == shell.uses()
            && self.edges.len() == shell.edges().len()
            && self
                .edges
                .iter()
                .zip(shell.edges())
                .all(|(a, b)| a.same_source_definition(b))
            && self.poles.len() == shell.poles().len()
            && self
                .poles
                .iter()
                .zip(shell.poles())
                .all(|((a, p), (b, q))| a == b && *p == q.point())
            && shell.regions().is_some_and(|regions| {
                self.regions.len() == regions.len()
                    && self
                        .regions
                        .iter()
                        .zip(regions)
                        .all(|(a, b)| a.same_definition(b))
            })
    }
}
