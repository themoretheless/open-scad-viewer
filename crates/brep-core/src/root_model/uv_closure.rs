//! Exact endpoint UV incidence. This is independent of Jordan/material proofs.
use super::*;

#[derive(Clone, Debug)]
pub struct QualifiedRootUvClosure {
    source: Arc<RootBrep>,
    max_joints: usize,
    joints: usize,
}
fn ordinary(port: &ss::QualifiedOriginalSsPort) -> bool {
    matches!(port.kind(), ss::OriginalSsPortKind::SameRootSelector | ss::OriginalSsPortKind::OriginalParameterRoot)
        && port.phase() == [0;4]
}
/// Prove a native UV joint; literal world incidence alone is insufficient.
pub fn qualify_root_uv_joint(a: &ss::QualifiedOriginalSsPort, support_a: usize,
    b: &ss::QualifiedOriginalSsPort, support_b: usize) -> Result<()> {
    if support_a >= 2 || support_b >= 2 || !a.point().same_selector(b.point()) {
        return Err(unproved("UV joint lacks valid supports or a literal shared selector"));
    }
    if a.same_third_surface_uv(b,support_a,support_b)? { return Ok(()); }
    for (triple, ts, plain, ps) in [(a,support_a,b,support_b),(b,support_b,a,support_a)] {
        if let Some((witness,roles))=triple.third_surface_binding() {
            let original=witness.source().original().source();
            let target=plain.target().source();
            if ordinary(plain) && roles[ts]==ps && roles[ts]<2
                && target.first()==original.first() && target.second()==original.second() {
                return Ok(());
            }
        }
    }
    let x=a.target().source();let y=b.target().source();
    if ordinary(a) && ordinary(b) && support_a==support_b
        && x.first()==y.first() && x.second()==y.second() {
        // Both endpoint binders preserve all native coordinates of the same
        // source point, including across different affine Cox spans/charts.
        return Ok(());
    }
    Err(unproved("Common-face native UV identity remains unproved"))
}
impl QualifiedRootUvClosure {
    pub fn qualify(source: Arc<RootBrep>, max_joints: usize) -> Result<Self> {
        if max_joints==0 || max_joints>262144 { return Err(unproved("Invalid UV closure joint budget")); }
        let t=source.topology();
        // Count before work; no partially checked face is returned.
        let mut joints=0usize;
        for face in &t.faces {
            for id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                joints=joints.checked_add(t.loops[id].coedges.len()).ok_or_else(||unproved("UV closure count overflow"))?;
                if joints>max_joints {return Err(unproved("UV closure joint budget exhausted"));}
            }
        }
        for face in &t.faces {
            for id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                let coedges=&t.loops[id].coedges;
                for i in 0..coedges.len() {
                    let a=&coedges[i];let b=&coedges[(i+1)%coedges.len()];
                    let ea=&t.edges[a.edge].curve;let eb=&t.edges[b.edge].curve;
                    let end=if a.reversed {ea.start()} else {ea.end()};
                    let start=if b.reversed {eb.end()} else {eb.start()};
                    qualify_root_uv_joint(end,a.pcurve.support,start,b.pcurve.support)?;
                }
            }
        }
        if joints==0 {return Err(unproved("UV closure has no face contours"));}
        Ok(Self {source,max_joints,joints})
    }
    pub fn joints(&self)->usize {self.joints}
    pub fn source(&self)->&Arc<RootBrep> {&self.source}
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;
        let fresh=Self::qualify(self.source.clone(),self.max_joints)?;
        if fresh.joints!=self.joints {return Err(unproved("UV closure incidence changed"));}
        Ok(())
    }
}
