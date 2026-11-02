//! Native cap chart, UV-region and continuous seam certificates. A bounded
//! nonzero seam is recorded, never promoted to exact source closure.
use super::*;
pub mod pullbacks;
use crate::profile_certificate::original_restriction::{
    self as exact, plane,
    wire::{self, jordan, nesting},
};
#[derive(Clone, Debug)]
pub struct Boundary {
    edge: usize,
    coedge: [usize; 2],
    proof: plane::Boundary,
}
impl Boundary {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn coedge(&self) -> [usize; 2] {
        self.coedge
    }
    pub fn proof(&self) -> &plane::Boundary {
        &self.proof
    }
}
#[derive(Clone, Debug)]
pub struct Cap {
    face: usize,
    reversed: bool,
    chart: plane::Chart,
    nesting: nesting::Nesting,
    boundaries: Vec<Boundary>,
}
impl Cap {
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn reversed(&self) -> bool {
        self.reversed
    }
    pub fn chart(&self) -> &plane::Chart {
        &self.chart
    }
    pub fn nesting(&self) -> &nesting::Nesting {
        &self.nesting
    }
    pub fn boundaries(&self) -> &[Boundary] {
        &self.boundaries
    }
}
#[derive(Clone, Debug)]
pub struct SourceCaps {
    source: Model,
    caps: Vec<Cap>,
}
#[derive(Clone, Debug)]
pub struct ExactCaps {
    caps: SourceCaps,
}
fn derive(source: &Model) -> Result<Vec<Cap>> {
    source.validate()?;
    if source.bodies.len() != 1 || source.shells.len() != 1 {
        return Err(error("Native cap qualification needs one source shell"));
    }
    let low = source
        .vertices
        .iter()
        .map(|v| v.point[2])
        .fold(f64::INFINITY, f64::min);
    let high = source
        .vertices
        .iter()
        .map(|v| v.point[2])
        .fold(f64::NEG_INFINITY, f64::max);
    if low >= high
        || source
            .vertices
            .iter()
            .any(|v| v.point[2] != low && v.point[2] != high)
    {
        return Err(error("Native source has no two-height cap frame"));
    }
    let mut caps = Vec::new();
    let mut ends = [0usize; 2];
    for usage in &source.shells[0].faces {
        let f = &source.faces[usage.face];
        let z = f.surface.control_points[0][0][2];
        if !f.surface.control_points.iter().flatten().all(|p| p[2] == z) {
            continue;
        }
        let end = if z == low {
            0
        } else if z == high {
            1
        } else {
            return Err(error("Native cap has an unexpected height"));
        };
        ends[end] += 1;
        let chart = plane::chart(&f.surface)?;
        let sign = chart.jacobian_sign() * if usage.reversed { -1 } else { 1 };
        if sign != if end == 0 { -1 } else { 1 } {
            return Err(error("Native cap has wrong physical normal orientation"));
        }
        let mut wires = Vec::new();
        let mut boundaries = Vec::new();
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            let mut original = Vec::new();
            for (ci, c) in source.loops[w].coedges.iter().enumerate() {
                if c.trim().is_some_and(|t| {
                    t.curve_parameter != source.edges[c.edge].curve.domain()
                        || t.pcurve_parameter != c.pcurve.domain()
                        || t.periodic_lift != [[0; 2]; 2]
                }) {
                    return Err(error(
                        "Native cap has an unsupported partial or lifted coedge",
                    ));
                }
                let world = raw_profiles::oriented(source, c.edge, c.reversed, z)?;
                let proof = plane::qualify(&f.surface, &c.pcurve, &world, source.tolerance_mm)?;
                original.push(exact::restrict_binary64(&c.pcurve, c.pcurve.domain())?);
                boundaries.push(Boundary {
                    edge: c.edge,
                    coedge: [w, ci],
                    proof,
                });
            }
            let mask = vec![false; original.len()];
            wires.push(wire::sew(original, mask, ["0".into(), "1".into()])?);
        }
        let jordan = jordan::qualify(&wires, jordan::Limits::default())?;
        let nesting = nesting::qualify(&jordan, nesting::Limits::default())?;
        caps.push(Cap {
            face: usage.face,
            reversed: usage.reversed,
            chart,
            nesting,
            boundaries,
        });
    }
    if ends != [1, 1] {
        return Err(error("Native source cap census is not complete"));
    }
    Ok(caps)
}
pub fn qualify(source: &Model) -> Result<SourceCaps> {
    Ok(SourceCaps {
        source: source.clone(),
        caps: derive(source)?,
    })
}
impl SourceCaps {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn caps(&self) -> &[Cap] {
        &self.caps
    }
    pub fn exact(&self) -> bool {
        self.caps
            .iter()
            .flat_map(|c| &c.boundaries)
            .all(|b| b.proof.exact())
    }
    pub fn require_exact(self) -> Result<ExactCaps> {
        if !self.exact() {
            return Err(error(
                "Native cap seams have nonzero certified displacement; exact closure is not proved",
            ));
        }
        Ok(ExactCaps { caps: self })
    }
    pub fn reverify(&self) -> Result<()> {
        let expected = derive(&self.source)?;
        if expected.len() != self.caps.len() {
            return Err(error("Native cap census changed"));
        }
        for (a, b) in self.caps.iter().zip(expected) {
            a.chart.reverify()?;
            if a.face != b.face
                || a.reversed != b.reversed
                || a.chart.surface() != b.chart.surface()
                || a.nesting.orientations() != b.nesting.orientations()
                || a.nesting.containment() != b.nesting.containment()
                || a.boundaries.len() != b.boundaries.len()
            {
                return Err(error("Native cap chart or UV region replay changed"));
            }
            a.nesting.reverify()?;
            for (a, b) in a.boundaries.iter().zip(b.boundaries) {
                a.proof.reverify()?;
                if a.edge != b.edge
                    || a.coedge != b.coedge
                    || a.proof.pcurve() != b.proof.pcurve()
                    || a.proof.world_curve() != b.proof.world_curve()
                    || a.proof.error_squared_bound() != b.proof.error_squared_bound()
                    || a.proof.cells() != b.proof.cells()
                {
                    return Err(error("Native cap boundary replay changed"));
                }
            }
        }
        Ok(())
    }
}
impl ExactCaps {
    pub fn caps(&self) -> &SourceCaps {
        &self.caps
    }
    pub fn reverify(&self) -> Result<()> {
        self.caps.reverify()?;
        if !self.caps.exact() {
            return Err(error("Exact native cap identity changed"));
        }
        Ok(())
    }
}
impl CurvedFullRound {
    pub fn qualify_source_caps(&self) -> Result<SourceCaps> {
        qualify(self.source())
    }
}
