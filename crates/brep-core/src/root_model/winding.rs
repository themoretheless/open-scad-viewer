//! Winding via convex homotopies of complete original UV arc enclosures.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive,Zero};
type Box2=[[f64;2];2];
#[derive(Clone,Debug)]
pub struct QualifiedRootLoopWinding {
    source:Arc<RootBrep>,face:usize,loop_in_face:usize,query:Box2,
    limits:ss::OriginalSsWindingLimits,winding:i32,segments:usize,
}
fn outward(q:&Q,lower:bool)->Result<f64> {
    let x=q.to_f64().filter(|x|x.is_finite()).ok_or_else(||unproved("UV clock overflow"))?;
    let exact=Q::from_float(x).unwrap();
    Ok(if lower && exact>*q {x.next_down()} else if !lower && exact<*q {x.next_up()} else {x})
}
fn charge(r:&mut ss::OriginalSsCurveLimits,b:&ss::OriginalSsCurveBounds)->Result<()> {
    r.native_rectangles=r.native_rectangles.checked_sub(b.native_rectangles).ok_or_else(||unproved("UV rectangle budget exhausted"))?;
    r.interval_operations=r.interval_operations.checked_sub(b.interval_operations).ok_or_else(||unproved("UV interval budget exhausted"))?;
    r.projection.max_work=r.projection.max_work.checked_sub(b.projection_work).ok_or_else(||unproved("UV projection budget exhausted"))?;
    r.world_rectangles=r.world_rectangles.checked_sub(b.world_rectangles).ok_or_else(||unproved("UV world budget exhausted"))?;
    Ok(())
}
fn project(curve:&ss::OriginalSsRootCurve,d:[f64;2],support:usize,r:&mut ss::OriginalSsCurveLimits)->Result<Box2> {
    let b=curve.enclose(d,*r)?;charge(r,&b)?;Ok([b.uv[2*support],b.uv[2*support+1]])
}
fn endpoint(port:&ss::QualifiedOriginalSsPort,support:usize,r:&mut ss::OriginalSsCurveLimits)->Result<Box2> {
    let p=port.parameter_bounds();let d=port.target().parameter_domain();
    project(port.target(),[outward(&p[0],true)?.max(d[0]),outward(&p[1],false)?.min(d[1])],support,r)
}
fn sample(b:Box2)->[f64;2] {b.map(|d|(d[0]*0.5+d[1]*0.5).max(d[0]).min(d[1]))}
fn sign(a:[f64;2],b:[f64;2],p:[f64;2])->Q {
    let q=|v|Q::from_float(v).unwrap();
    (q(b[0])-q(a[0]))*(q(p[1])-q(a[1]))-(q(b[1])-q(a[1]))*(q(p[0])-q(a[0]))
}
impl QualifiedRootLoopWinding {
    pub fn winding(&self)->i32 {self.winding}
    pub fn segments(&self)->usize {self.segments}
    pub fn qualify(source:Arc<RootBrep>,face:usize,loop_in_face:usize,query:Box2,limits:ss::OriginalSsWindingLimits)->Result<Self> {
        if !query.iter().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<=d[1]) || !(1..=4096).contains(&limits.max_segments) {
            return Err(unproved("Invalid mixed-loop winding query or budget"));
        }
        let t=source.topology();let f=t.faces.get(face).ok_or_else(||unproved("Missing winding face"))?;
        let id=if loop_in_face==0 {f.outer} else {*f.holes.get(loop_in_face-1).ok_or_else(||unproved("Missing winding loop"))?};
        let uses=&t.loops[id].coedges;
        if uses.is_empty() {return Err(unproved("Empty winding contour"));}
        let mut remaining=limits.geometry;let mut starts=Vec::new();
        // A single box/sample at each joint makes the temporary polygon
        // connected even when adjacent SS pairs have different UV enclosures.
        for i in 0..uses.len() {
            let a=&uses[(i+uses.len()-1)%uses.len()];let b=&uses[i];
            let ea=&t.edges[a.edge].curve;let eb=&t.edges[b.edge].curve;
            let end=if a.reversed {ea.start()} else {ea.end()};
            let start=if b.reversed {eb.end()} else {eb.start()};
            qualify_root_uv_joint(end,a.pcurve.support,start,b.pcurve.support)?;
            let x=endpoint(end,a.pcurve.support,&mut remaining)?;let y=endpoint(start,b.pcurve.support,&mut remaining)?;
            let joint:Box2=std::array::from_fn(|k|[x[k][0].max(y[k][0]),x[k][1].min(y[k][1])]);
            if joint.iter().any(|d|d[0]>d[1]) {return Err(unproved("Proved UV joint enclosures are inconsistent"));}
            starts.push(joint);
        }
        let point=sample(query);let mut winding=0;let mut segments=0;
        for (i,c) in uses.iter().enumerate() {
            let e=&t.edges[c.edge].curve;
            let (a,b)=if c.reversed {(e.end(),e.start())} else {(e.start(),e.end())};
            let forward=e.forward()!=c.reversed;
            let (lo,hi)=if forward {(&a.parameter_bounds()[1],&b.parameter_bounds()[0])} else {(&b.parameter_bounds()[1],&a.parameter_bounds()[0])};
            let p=e.parameter_bounds();let natural=e.curve().parameter_domain();
            let low=outward(&p[0],true)?.max(natural[0]);let high=outward(&p[1],false)?.min(natural[1]);
            let (c0,c1)=if forward {(low,high)} else {(high,low)};
            let mut pending=vec![(c0,c1,starts[i],starts[(i+1)%uses.len()],lo.clone(),hi.clone())];
            while let Some((c0,c1,start,end,lo,hi))=pending.pop() {
                if segments==limits.max_segments {return Err(unproved("Mixed-loop winding subdivision exhausted"));}segments+=1;
                let arc=project(e.curve(),[c0.min(c1),c0.max(c1)],c.pcurve.support,&mut remaining)?;
                let hull:Box2=std::array::from_fn(|k|[arc[k][0].min(start[k][0]).min(end[k][0]),arc[k][1].max(start[k][1]).max(end[k][1])]);
                if !(0..2).any(|k|query[k][1]<hull[k][0] || hull[k][1]<query[k][0]) {
                    let q=(&lo+&hi)/Q::from_integer(2.into());let x=outward(&q,true)?;let xq=Q::from_float(x).unwrap();
                    if !(lo<xq && xq<hi && c0.min(c1)<x && x<c0.max(c1)) {return Err(unproved("Mixed winding has no strict interior clock"));}
                    let mid=project(e.curve(),[x;2],c.pcurve.support,&mut remaining)?;
                    let (left,right)=if forward {((lo,xq.clone()),(xq,hi))} else {((xq.clone(),hi),(lo,xq))};
                    pending.push((x,c1,mid,end,right.0,right.1));pending.push((c0,x,start,mid,left.0,left.1));continue;
                }
                let a=sample(start);let b=sample(end);let s=sign(a,b,point);
                if a[1]<=point[1] && point[1]<b[1] && s>Q::zero() {winding+=1;}
                if b[1]<=point[1] && point[1]<a[1] && s<Q::zero() {winding-=1;}
            }
        }
        Ok(Self {source,face,loop_in_face,query,limits,winding,segments})
    }
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;
        let fresh=Self::qualify(self.source.clone(),self.face,self.loop_in_face,self.query,self.limits)?;
        if self.winding!=fresh.winding || self.segments!=fresh.segments {return Err(unproved("Mixed winding proof changed"));}Ok(())
    }
}
