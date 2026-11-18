//! Explicit replacement for auxiliary UV boundaries. Original carriers and
//! world rims remain primary; the legacy pcurves and seam receipt are retained.
use super::*;
#[derive(Clone, Debug)]
pub struct BoundaryPullback {
    edge: usize,
    coedge: [usize; 2],
    inverse: plane::pullback::Pullback,
}
impl BoundaryPullback {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn coedge(&self) -> [usize; 2] {
        self.coedge
    }
    pub fn inverse(&self) -> &plane::pullback::Pullback {
        &self.inverse
    }
}
#[derive(Clone, Debug)]
pub struct CapPullbacks {
    face: usize,
    boundaries: Vec<BoundaryPullback>,
    world_region: nesting::Nesting,
}
impl CapPullbacks {
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn boundaries(&self) -> &[BoundaryPullback] {
        &self.boundaries
    }
    pub fn world_region(&self) -> &nesting::Nesting {
        &self.world_region
    }
}
#[derive(Clone, Debug)]
pub struct CanonicalPullbacks {
    original: SourceCaps,
    caps: Vec<CapPullbacks>,
}
fn derive(original: &SourceCaps) -> Result<Vec<CapPullbacks>> {
    original.reverify()?;
    original
        .caps
        .iter()
        .map(|cap| {
            let boundaries = cap
                .boundaries
                .iter()
                .map(|b| {
                    Ok(BoundaryPullback {
                        edge: b.edge,
                        coedge: b.coedge,
                        inverse: plane::pullback::qualify(
                            cap.chart.surface(),
                            b.proof.world_curve(),
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let face = &original.source.faces[cap.face];
            let mut wires = Vec::new();
            for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                let mut curves = boundaries
                    .iter()
                    .filter(|b| b.coedge[0] == loop_id)
                    .map(|b| b.inverse.world_curve())
                    .collect::<Vec<_>>();
                let reverse = cap.chart.jacobian_sign() < 0;
                if reverse {
                    curves.reverse();
                }
                let pieces = curves
                    .into_iter()
                    .map(|c| {
                        let mut domain = c.domain();
                        if reverse {
                            domain.reverse();
                        }
                        exact::restrict_binary64(c, domain)
                    })
                    .collect::<Result<Vec<_>>>()?;
                let mask = vec![false; pieces.len()];
                wires.push(wire::sew(pieces, mask, ["0".into(), "1".into()])?);
            }
            let jordan = jordan::qualify(&wires, jordan::Limits::default())?;
            let world_region = nesting::qualify(&jordan, nesting::Limits::default())?;
            Ok(CapPullbacks {
                face: cap.face,
                boundaries,
                world_region,
            })
        })
        .collect()
}
impl SourceCaps {
    pub fn canonical_pullbacks(&self) -> Result<CanonicalPullbacks> {
        Ok(CanonicalPullbacks {
            original: self.clone(),
            caps: derive(self)?,
        })
    }
}
impl CanonicalPullbacks {
    /// Preserved original UV data, including its nonzero seam bounds.
    pub fn original(&self) -> &SourceCaps {
        &self.original
    }
    pub fn caps(&self) -> &[CapPullbacks] {
        &self.caps
    }
    pub fn reverify(&self) -> Result<()> {
        let expected = derive(&self.original)?;
        if self.caps.len() != expected.len() {
            return Err(error("Canonical cap census changed"));
        }
        for (a, b) in self.caps.iter().zip(expected) {
            a.world_region.reverify()?;
            if a.face != b.face
                || a.boundaries.len() != b.boundaries.len()
                || a.world_region.orientations() != b.world_region.orientations()
                || a.world_region.containment() != b.world_region.containment()
            {
                return Err(error("Canonical cap world region changed"));
            }
            for (a, b) in a.boundaries.iter().zip(b.boundaries) {
                a.inverse.reverify()?;
                if a.edge != b.edge
                    || a.coedge != b.coedge
                    || a.inverse.chart().surface() != b.inverse.chart().surface()
                    || a.inverse.world_curve() != b.inverse.world_curve()
                {
                    return Err(error("Canonical cap original boundary changed"));
                }
            }
        }
        Ok(())
    }
}
