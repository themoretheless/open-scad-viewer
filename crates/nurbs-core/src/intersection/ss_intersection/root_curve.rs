//! Exact root-valued SS geometry. Neither the world edge nor either UV curve
//! is replaced by fitted rational poles. Numerical queries return conservative
//! original-source enclosures; they never become a new geometry definition.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
use std::sync::Arc;
mod frames;
mod window;
mod joint;
pub use joint::QualifiedOriginalSsJoint;
pub use window::QualifiedOriginalSsGraphWindow;
mod query;
#[derive(Clone, Debug)]
enum Carrier {
    Native(QualifiedFiber),
    Quotient(QualifiedQuotientFiber),
    Deflated(QualifiedDeflatedSsFiber),
    Pole(QualifiedPoleSsRootArc),
    Blowup(QualifiedSsBlowupRootArc),
}
#[derive(Clone, Debug)]
pub struct OriginalSsRootCurve {
    carrier: Carrier,
}
#[derive(Clone, Copy, Debug)]
pub struct OriginalSsCurveLimits {
    pub max_refinements: usize,
    pub native_rectangles: usize,
    pub interval_operations: usize,
    pub projection: ExactSsAlgebraLimits,
    pub world_rectangles: usize,
}
impl Default for OriginalSsCurveLimits {
    fn default() -> Self {
        Self {
            max_refinements: 8,
            native_rectangles: 100000,
            interval_operations: 1000000,
            projection: Default::default(),
            world_rectangles: 100000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OriginalSsCurveBounds {
    pub uv: [[f64; 2]; 4],
    pub world: [[f64; 2]; 3],
    pub native_rectangles: usize,
    pub interval_operations: usize,
    pub projection_work: usize,
    pub world_rectangles: usize,
}
impl OriginalSsRootCurve {
    /// Equality of the owned original root graphs, not approximate curve data.
    pub fn same_definition(&self, other: &Self) -> Result<bool> {
        if self.source().first() != other.source().first()
            || self.source().second() != other.source().second()
            || self.free_axis() != other.free_axis()
            || self.coordinate_domain() != other.coordinate_domain()
        {
            return Ok(false);
        }
        if matches!(self.carrier, Carrier::Pole(_)) != matches!(other.carrier, Carrier::Pole(_)) {
            return Ok(false);
        }
        match (&self.carrier,&other.carrier) {
            (Carrier::Blowup(a),Carrier::Blowup(b)) => Ok(a.source().original_coordinate_map()==b.source().original_coordinate_map()
                && a.source().equations()==b.source().equations()
                && a.source().node().source().original().original().spans()==b.source().node().source().original().original().spans()),
            (Carrier::Blowup(_),_)|(_,Carrier::Blowup(_))=>Ok(false),
            _=>Ok(frames::frame(self)? == frames::frame(other)?),
        }
    }

    pub fn from_native(fiber: QualifiedFiber) -> Self {
        Self {
            carrier: Carrier::Native(fiber),
        }
    }
    pub fn from_quotient(fiber: QualifiedQuotientFiber) -> Self {
        Self {
            carrier: Carrier::Quotient(fiber),
        }
    }
    pub fn from_deflated(fiber: QualifiedDeflatedSsFiber) -> Self {
        Self {
            carrier: Carrier::Deflated(fiber),
        }
    }
    pub fn from_pole(fiber: QualifiedPoleSsRootArc) -> Self {
        Self {
            carrier: Carrier::Pole(fiber),
        }
    }
    pub fn from_blowup(fiber:QualifiedSsBlowupRootArc)->Self {Self {carrier:Carrier::Blowup(fiber)}}
    pub fn node_at(&self,parameter:f64)->Option<&QualifiedSsRootNode> {
        match &self.carrier {Carrier::Blowup(f) if f.node_parameter()==Some(parameter)=>Some(f.source().node()),_=>None}
    }
    pub fn source(&self) -> &FiberSources {
        match &self.carrier {
            Carrier::Native(f) => f.source(),
            Carrier::Quotient(f) => f.source(),
            Carrier::Deflated(f) => f.source().original().original().source(),
            Carrier::Pole(f) => f.source().original().original().original().source(),
            Carrier::Blowup(f) => f.source().node().source().original().original().source(),
        }
    }
    pub fn free_axis(&self) -> FiberAxis {
        match &self.carrier {
            Carrier::Native(f) => f.free_axis(),
            Carrier::Quotient(f) => f.free_axis(),
            Carrier::Deflated(f) => f.free_axis(),
            Carrier::Pole(f) => f.free_axis(),
            Carrier::Blowup(f) => f.free_axis(),
        }
    }
    pub fn coordinate_domain(&self) -> [[f64; 2]; 4] {
        match &self.carrier {
            Carrier::Native(f) => f.domain(),
            Carrier::Quotient(f) => f.domain(),
            Carrier::Deflated(f) => f.domain(),
            Carrier::Pole(f) => f.domain(),
            Carrier::Blowup(f) => f.domain(),
        }
    }
    pub fn parameter_domain(&self) -> [f64; 2] {
        self.coordinate_domain()[self.free_axis().index()]
    }
    pub fn reverify(&self) -> Result<()> {
        match &self.carrier {
            Carrier::Native(f) => f.reverify(),
            Carrier::Quotient(f) => f.reverify(),
            Carrier::Deflated(f) => f.reverify(),
            Carrier::Pole(f) => f.reverify(),
            Carrier::Blowup(f) => f.reverify(),
        }
    }
    /// Both pcurves are the exact original root projections; world is S1(root)
    /// =S2(root). A pole endpoint retains its exact original constant image.
    pub fn enclose(
        &self,
        parameter: [f64; 2],
        limits: OriginalSsCurveLimits,
    ) -> Result<OriginalSsCurveBounds> {
        query::enclose(self, parameter, limits)
    }
    pub(crate) fn original_deflated_span_ideal(&self)->Option<&QualifiedOriginalSsIdeal> {
        match &self.carrier {Carrier::Deflated(f)=>Some(f.source().original().original()),_=>None}
    }
    /// Necessary candidate clock constraints from a third original surface.
    /// This does not certify bounded third-UV membership or a trim vertex.
    pub fn third_surface_candidates(&self, third:&Surface, spans:[usize;2], limits:ExactSsGroebnerLimits)->Result<QualifiedSsThirdSurfaceCandidates> {
        let ideal=match &self.carrier {
            Carrier::Deflated(f)=>f.source().original().original(),
            Carrier::Pole(f)=>f.source().original().original().original(),
            _=>return Err(crate::input("Third-surface candidates require an original affine span graph")),
        };
        qualify_ss_third_surface_candidates(ideal,third,spans,self.free_axis().index(),limits)
    }
    pub fn pole_endpoint(&self) -> Option<(f64, &[Q; 3])> {
        match &self.carrier {
            Carrier::Pole(f) => Some((f.source().pole().shore() as f64, f.pole_world_point())),
            _ => None,
        }
    }
}
mod point;
pub use point::OriginalSsRootPoint;

mod port;
pub use port::{OriginalSsPortKind, QualifiedOriginalSsPort, bind_original_ss_port};
mod wire;
pub use wire::{OriginalSsRootEdge,OriginalSsRootWire};
pub use wire::{OriginalSsWindingLimits,QualifiedOriginalSsWinding,qualify_original_ss_winding};
pub use wire::{OriginalSsPathLimits,QualifiedOriginalSsPathParity,qualify_original_ss_path_parity};

mod algebraic_clock;
pub use algebraic_clock::{SsAlgebraicRootCensusLimits,QualifiedSsAlgebraicRootCensus,SsAlgebraicClockLimits,SsAlgebraicClockEnclosure,QualifiedSsAlgebraicClock};
mod algebraic_point;
pub use algebraic_point::{SsAlgebraicPointLimits,SsAlgebraicPointBounds,OriginalSsAlgebraicPoint};

mod algebraic_port;
pub use algebraic_port::{QualifiedSsAlgebraicPort,OriginalSsAlgebraicEdge};

mod partition;
pub use partition::{SsRootEdgePartitionLimits,QualifiedOriginalSsEdgePartition};
