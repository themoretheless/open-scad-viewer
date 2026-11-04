//! Root-valued edge restrictions and literal closed world incidence. This is
//! not an embedded Jordan contour or a retained material/solid certificate.
use super::*;
use num_traits::ToPrimitive;
#[derive(Clone, Debug)]
pub struct OriginalSsRootEdge {
    start: QualifiedOriginalSsPort,
    end: QualifiedOriginalSsPort,
    forward: bool,
}
impl OriginalSsRootEdge {
    pub fn new(start: QualifiedOriginalSsPort, end: QualifiedOriginalSsPort) -> Result<Self> {
        check(
            start.target().same_definition(end.target())?,
            "Root edge endpoints belong to different original root graphs",
        )?;
        let a = start.parameter_bounds();
        let b = end.parameter_bounds();
        let forward = if a[1] < b[0] {
            true
        } else {
            check(
                b[1] < a[0],
                "Root-valued edge endpoint order remains unproved",
            )?;
            false
        };
        Ok(Self {
            start,
            end,
            forward,
        })
    }
    pub fn start(&self) -> &QualifiedOriginalSsPort {
        &self.start
    }
    pub fn end(&self) -> &QualifiedOriginalSsPort {
        &self.end
    }
    pub fn curve(&self) -> &Arc<OriginalSsRootCurve> {
        self.start.target()
    }
    pub fn forward(&self) -> bool {
        self.forward
    }
    pub fn parameter_bounds(&self) -> [Q; 2] {
        let a = self.start.parameter_bounds();
        let b = self.end.parameter_bounds();
        [
            a[0].clone().min(b[0].clone()),
            a[1].clone().max(b[1].clone()),
        ]
    }
    pub fn enclose(&self, limits: OriginalSsCurveLimits) -> Result<OriginalSsCurveBounds> {
        let bounds = self.parameter_bounds();
        let natural = self.curve().parameter_domain();
        let mut d = [0.; 2];
        for k in 0..2 {
            let x = bounds[k]
                .to_f64()
                .filter(|x| x.is_finite())
                .ok_or_else(|| crate::input("Root-valued edge clock overflow"))?;
            let q = Q::from_float(x).unwrap();
            d[k] = if k == 0 {
                if q > bounds[k] { x.next_down() } else { x }
            } else {
                if q < bounds[k] { x.next_up() } else { x }
            };
        }
        d[0] = d[0].max(natural[0]);
        d[1] = d[1].min(natural[1]);
        self.curve().enclose(d, limits)
    }
    pub fn reverify(&self) -> Result<()> {
        self.start.reverify()?;
        self.end.reverify()?;
        let fresh = Self::new(self.start.clone(), self.end.clone())?;
        check(
            fresh.forward == self.forward,
            "Original root edge direction changed",
        )
    }
}
#[derive(Clone, Debug)]
pub struct OriginalSsRootWire {
    edges: Vec<OriginalSsRootEdge>,
}
impl OriginalSsRootWire {
    pub fn new(edges: Vec<OriginalSsRootEdge>) -> Result<Self> {
        check(
            (1..=256).contains(&edges.len()),
            "Original root wire requires bounded nonempty edges",
        )?;
        for i in 0..edges.len() {
            check(
                edges[i]
                    .end()
                    .point()
                    .same_selector(edges[(i + 1) % edges.len()].start().point()),
                "Original root wire is not closed by shared exact selectors",
            )?;
        }
        Ok(Self { edges })
    }
    /// Reconcile independent endpoint owners by exact original graph/root
    /// identity, then retain ordinary literal incidence in the returned wire.
    pub fn reconcile_selectors(edges: Vec<OriginalSsRootEdge>, limits: SsAlgebraicClockLimits) -> Result<Self> {
        check((1..=256).contains(&edges.len()),"SS reconciliation requires bounded nonempty edges")?;
        let mut joined=Vec::with_capacity(edges.len());
        for i in 0..edges.len() {
            let edge=&edges[i];
            let next=&edges[(i+1)%edges.len()];
            let end=if edge.end().point().same_selector(next.start().point()) {
                edge.end().clone()
            } else {edge.end().rebind_equivalent(next.start().point().clone(),limits)?};
            joined.push(OriginalSsRootEdge::new(edge.start().clone(),end)?);
        }
        Self::new(joined)
    }
    pub fn edges(&self) -> &[OriginalSsRootEdge] {
        &self.edges
    }
    pub fn reverify(&self) -> Result<()> {
        for edge in &self.edges {
            edge.reverify()?;
        }
        Self::new(self.edges.clone()).map(|_| ())
    }
}
mod winding;
pub use winding::{OriginalSsWindingLimits,QualifiedOriginalSsWinding,qualify_original_ss_winding};
mod path_parity;
pub use path_parity::{OriginalSsPathLimits,QualifiedOriginalSsPathParity,qualify_original_ss_path_parity};
impl OriginalSsRootWire {
    /// One winding for every point of a closed UV query box, not just its
    /// midpoint. Entire original arcs must be separated from the whole box.
    pub fn winding_box(&self,support:usize,query:[[f64;2];2],limits:OriginalSsWindingLimits)->Result<QualifiedOriginalSsWinding> {
        winding::qualify_box(self,support,query,limits)
    }
}
