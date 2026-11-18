//! Explicit legacy field names/order/optional-field semantics.
use super::*;
use crate::{
    ChangeKind, ChangeProvenance, ChangeSet, Curve, Surface, TopoId, TopoKind, TopologyChange,
    TopologyIds, TopologyLineageRecord,
};
use brep_topology::{Body, Coedge, CoedgeTrim, Edge, Face, FaceUse, Loop, Shell, Vertex};
macro_rules! fields {($t:ty,$($key:literal=>$field:ident),*$(,)?)=>{impl Put for $t{fn put(&self,e:&mut Encoder,d:usize)->Result<()>{object!(e,d,$($key=>&self.$field),*)}}}}
fields!(Curve,"degree"=>degree,"knots"=>knots,"controlPoints"=>control_points,"weights"=>weights,"periodic"=>periodic);
fields!(Surface,"degreeU"=>degree_u,"degreeV"=>degree_v,"knotsU"=>knots_u,"knotsV"=>knots_v,"controlPoints"=>control_points,"weights"=>weights,"periodicU"=>periodic_u,"periodicV"=>periodic_v);
fields!(Vertex,"point"=>point);
fields!(CoedgeTrim,"curveParameter"=>curve_parameter,"pcurveParameter"=>pcurve_parameter,"periodicLift"=>periodic_lift);
fields!(Loop<Curve>,"coedges"=>coedges);
fields!(Face<Surface>,"surface"=>surface,"outer"=>outer,"holes"=>holes);
fields!(FaceUse,"face"=>face,"reversed"=>reversed);
fields!(Shell,"faces"=>faces,"closed"=>closed);
fields!(Body,"outerShell"=>outer_shell,"innerShells"=>inner_shells);
fields!(SourceMesh,"positions"=>positions,"indices"=>indices);
fields!(TopologyLineageRecord,"operation"=>operation,"entityKind"=>entity_kind,"parents"=>parents,"children"=>children);
fields!(ChangeProvenance,"operation"=>operation,"operand"=>operand,"occurrence"=>occurrence);
fields!(TopologyChange,"kind"=>kind,"topoKind"=>topo_kind,"parents"=>parents,"children"=>children,"provenance"=>provenance,"role"=>role,"anchor"=>anchor);
impl Put for Edge<Curve> {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        let mut fields: Vec<(&str, &dyn Put)> =
            vec![("curve", &self.curve), ("vertices", &self.vertices)];
        if self.degenerate {
            fields.push(("degenerate", &self.degenerate))
        }
        e.object(d, &mut fields)
    }
}
impl Put for Coedge<Curve> {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        let trim = self.trim();
        let mut fields: Vec<(&str, &dyn Put)> = vec![
            ("edge", &self.edge),
            ("reversed", &self.reversed),
            ("pcurve", &self.pcurve),
        ];
        if let Some(ref t) = trim {
            fields.push(("trim", t))
        }
        e.object(d, &mut fields)
    }
}
impl Put for Source {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        match self {
            Source::Mesh(m) => object!(e,d,"kind"=>&"mesh","mesh"=>m),
            Source::Brep(m) => object!(e,d,"kind"=>&"brep","model"=>m),
        }
    }
}
impl Put for TopoId {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        self.to_string().put(e, d)
    }
}
impl Put for TopoKind {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        self.as_str().put(e, d)
    }
}
impl Put for ChangeKind {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        self.as_str().put(e, d)
    }
}
struct NamingNode<'a>(&'a TopoId, &'a TopoKind);
impl Put for NamingNode<'_> {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        object!(e,d,"id"=>self.0,"kind"=>self.1)
    }
}
impl Put for ChangeSet {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        let nodes = self
            .nodes
            .iter()
            .map(|(id, k)| NamingNode(id, k))
            .collect::<Vec<_>>();
        object!(e,d,"schema"=>&1u64,"nodes"=>&nodes,"changes"=>&self.changes)
    }
}
impl Put for TopologyIds {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        let tolerances = self
            .local_tolerances_mm
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<BTreeMap<_, _>>();
        let sewed = self.sewed_edges.iter().copied().collect::<Vec<_>>();
        object!(e,d,"vertices"=>&self.vertices,"edges"=>&self.edges,"loops"=>&self.loops,"faces"=>&self.faces,"shells"=>&self.shells,"bodies"=>&self.bodies,"lineage"=>&self.lineage,"changeSet"=>&self.change_set,"localTolerancesMm"=>&tolerances,"sewedEdges"=>&sewed)
    }
}
impl Put for BTreeMap<String, f64> {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        let mut fields = self
            .iter()
            .map(|(k, v)| (k.as_str(), v as &dyn Put))
            .collect::<Vec<_>>();
        e.object(d, &mut fields)
    }
}
impl Put for Model {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        let context =
            cad_predicates::ToleranceContext::from_brep_tolerance_mm(self.tolerance_mm).ok();
        let mut fields: Vec<(&str, &dyn Put)> = vec![
            ("vertices", &self.vertices),
            ("edges", &self.edges),
            ("loops", &self.loops),
            ("faces", &self.faces),
            ("shells", &self.shells),
            ("bodies", &self.bodies),
            ("toleranceMm", &self.tolerance_mm),
            ("topologyIds", &self.1),
        ];
        if let Some(ref c) = context {
            fields.push(("toleranceContext", c))
        }
        e.object(d, &mut fields)
    }
}
impl Put for cad_predicates::ToleranceContext {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        {
            let identity = self.spec_identity();
            object!(e,d,"spec"=>self.specification(),"identity"=>&identity)
        }
    }
}
fields!(cad_predicates::ToleranceSpecIdentity,"version"=>version,"canonical"=>canonical);
impl Put for cad_predicates::ToleranceSpec {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        object!(e,d,"version"=>&cad_predicates::TOLERANCE_SPEC_VERSION,"linearAbsMm"=>&self.linear_abs,"linearRelative"=>&self.linear_rel,"onMm"=>&self.on_tol,"clearMm"=>&self.clear_tol,"angularRadians"=>&self.angular,"parametricFloor"=>&self.param_floor,"ulpGuard"=>&self.ulp_guard,"maxEntityErrorMm"=>&self.max_entity_error,"policy"=>&self.policy)
    }
}
