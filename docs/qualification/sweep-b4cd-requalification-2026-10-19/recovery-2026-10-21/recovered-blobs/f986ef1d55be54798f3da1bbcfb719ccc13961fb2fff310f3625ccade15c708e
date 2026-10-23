//! Rational homogeneous Hermite bridges between any selected surface boundaries.
//! Matches source parametric jets in real arithmetic under an explicit affine
//! U correspondence. Rounding, regularity and global injectivity are separate.
use crate::{Result,check,numeric,curve::Curve,surface::Surface};
#[path="surface_blend/traces.rs"]
mod traces;
pub use traces::{Trace,TraceReport,g2_traces};
#[path="surface_blend/plane_trace.rs"]
mod plane_trace;
pub use plane_trace::g2_to_plane_trace;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Boundary { UStart, UEnd, VStart, VEnd }
impl Boundary {
    fn end(self)->bool {matches!(self,Self::UEnd|Self::VEnd)}
    fn along_u(self)->bool {matches!(self,Self::UStart|Self::UEnd)}
}
#[derive(Clone,Copy,Debug)]
pub struct Options {
    pub boundaries:[Boundary;2],
    /// Positive source-transverse units per blend-V unit; orientation supplies sign.
    pub gains:[f64;2], pub reverse_u:[bool;2], pub max_patches:usize,
}
impl Default for Options {fn default()->Self {Self {boundaries:[Boundary::VEnd,Boundary::VStart],gains:[1.;2],reverse_u:[false;2],max_patches:256}}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Order { C1, C2 }
#[derive(Clone,Debug)]
pub struct Report {
    pub patches:Vec<Surface>,
    /// Requested homogeneous jet matching order in exact real arithmetic.
    /// This is not a rounding-inclusive continuity or regularity certificate.
    pub real_arithmetic_order:Order,
}
pub(crate) fn canonical(s:&Surface,b:Boundary)->Result<Surface> {
    s.validate()?;
    if !b.along_u() {return Ok(s.clone());}
    let nu=s.control_points.len();let nv=s.control_points[0].len();
    Ok(Surface {degree_u:s.degree_v,degree_v:s.degree_u,knots_u:s.knots_v.clone(),knots_v:s.knots_u.clone(),
        control_points:(0..nv).map(|j|(0..nu).map(|i|s.control_points[i][j].clone()).collect()).collect(),
        weights:(0..nv).map(|j|(0..nu).map(|i|s.weights[i][j]).collect()).collect(),periodic_u:s.periodic_v,periodic_v:s.periodic_u})
}
/// Pack homogeneous boundary jets into polynomial XYZ curves: [Hx,Hy,Hw]
/// and [Hz,0,0]. Their unit weights allow exact-real basis alignment without
/// treating Cartesian vector fields with different rational denominators as
/// if they shared the boundary's denominator.
pub(crate) fn fields(s:&Surface,end:bool,gain:f64,reverse:bool,order:usize)->Result<Vec<Curve>> {
    s.validate()?;
    let nv=s.control_points[0].len();
    let v=if end {s.knots_v[nv]} else {s.knots_v[s.degree_v]};
    fields_at(s,v,gain,reverse,order)
}
pub(crate) fn fields_at(s:&Surface,v:f64,gain:f64,reverse:bool,order:usize)->Result<Vec<Curve>> {
    s.validate()?;
    let nv=s.control_points[0].len();
    let basis=crate::curve::basis(s.degree_v,&s.knots_v,nv,v,s.periodic_v)?;
    let derivatives=[basis.basis,basis.d1,basis.d2];
    let scale=s.weights.iter().flatten().copied().fold(0_f64,f64::max);
    let mut result=Vec::with_capacity((order+1)*2);
    for k in 0..=order {
        let factor=if k==0 {1.} else if k==1 {gain} else {gain*gain};
        numeric(factor.is_finite(),"Blend transverse scale overflow")?;
        let mut h=Vec::with_capacity(s.control_points.len());
        for (row,weights) in s.control_points.iter().zip(&s.weights) {
            let mut value=[0.;4];
            for j in 0..nv {
                let w=weights[j]/scale;
                numeric(w>0.&&w.is_finite(),"Blend weight normalization underflow")?;
                let c=derivatives[k][j]*factor*w;
                for axis in 0..3 {value[axis]+=c*row[j][axis];}value[3]+=c;
            }
            numeric(value.iter().all(|v|v.is_finite()),"Blend boundary jet overflow")?;h.push(value);
        }
        for packed in 0..2 {
            let points=h.iter().map(|v|if packed==0 {vec![v[0],v[1],v[3]]} else {vec![v[2],0.,0.]}).collect();
            let mut c=Curve {degree:s.degree_u,knots:s.knots_u.clone(),control_points:points,weights:vec![1.;h.len()],periodic:s.periodic_u};
            c.validate()?;if reverse {c=c.reverse()?;}result.push(c);
        }
    }
    Ok(result)
}
fn build(a:&Surface,b:&Surface,o:Options,order:Order)->Result<Report> {
    check(o.gains.iter().all(|v|v.is_finite()&&*v>0.)&&(1..=256).contains(&o.max_patches),"Blend requires positive finite gains and 1..256 patches")?;
    let a=canonical(a,o.boundaries[0])?;let b=canonical(b,o.boundaries[1])?;
    let ae=o.boundaries[0].end();let be=o.boundaries[1].end();
    let q=if order==Order::C1 {1} else {2};
    let mut input=fields(&a,ae,if ae {o.gains[0]} else {-o.gains[0]},o.reverse_u[0],q)?;
    input.extend(fields(&b,be,if be {-o.gains[1]} else {o.gains[1]},o.reverse_u[1],q)?);
    build_fields(input,order,o.max_patches)
}
fn build_fields(input:Vec<Curve>,order:Order,max_patches:usize)->Result<Report> {
    let q=if order==Order::C1 {1} else {2};
    let aligned=crate::sections::compatible(&input)?;
    let pieces=aligned.iter().map(Curve::decompose).collect::<Result<Vec<_>>>()?;
    let count=pieces[0].len();
    check(count<=max_patches,"Blend patch budget exceeded")?;
    check(pieces.iter().all(|p|p.len()==count),"Blend aligned fields have inconsistent span counts")?;
    let vdegree=if q==1 {3} else {5};let mut patches=Vec::with_capacity(count);
    let knots_v:Vec<f64>=std::iter::repeat_n(0.,vdegree+1).chain(std::iter::repeat_n(1.,vdegree+1)).collect();
    for index in 0..count {
        let domain=pieces[0][index].domain();
        check(pieces.iter().all(|p|p[index].domain()==domain),"Blend field spans do not match")?;
        let base=pieces[0][index].definition();let n=base.control_points.len();
        let h=|side:usize,jet:usize,i:usize| {
            let offset=side*2*(q+1)+jet*2;
            let xyw=&pieces[offset][index].definition().control_points[i];
            let z=pieces[offset+1][index].definition().control_points[i][0];
            [xyw[0],xyw[1],z,xyw[2]]
        };
        let mut controls=Vec::with_capacity(n);let mut weights=Vec::with_capacity(n);
        for i in 0..n {
            let a=h(0,0,i);let da=h(0,1,i);let b=h(1,0,i);let db=h(1,1,i);
            let mut row=vec![[0.;4];vdegree+1];row[0]=a;row[vdegree]=b;
            for k in 0..4 {
                row[1][k]=a[k]+da[k]/vdegree as f64;
                row[vdegree-1][k]=b[k]-db[k]/vdegree as f64;
            }
            if q==2 {
                let dda=h(0,2,i);let ddb=h(1,2,i);
                for k in 0..4 {row[2][k]=a[k]+2.*da[k]/5.+dda[k]/20.;row[3][k]=b[k]-2.*db[k]/5.+ddb[k]/20.;}
            }
            check(row.iter().all(|p|p[3].is_finite()&&p[3]>0.),"Blend has nonpositive homogeneous weights; reduce transverse gains")?;
            controls.push(row.iter().map(|p|vec![p[0]/p[3],p[1]/p[3],p[2]/p[3]]).collect());
            weights.push(row.iter().map(|p|p[3]).collect());
        }
        let s=Surface {degree_u:base.degree,degree_v:vdegree,knots_u:base.knots.clone(),knots_v:knots_v.clone(),control_points:controls,weights,periodic_u:false,periodic_v:false};
        s.validate()?;patches.push(s);
    }
    Ok(Report {patches,real_arithmetic_order:order})
}
/// Cubic bridge matching source boundary positions and first transverse jets.
/// C1 implies G1 wherever source/bridge charts are regular. U is normalized
/// on each source independently; no nonlinear correspondence is inferred.
pub fn g1(a:&Surface,b:&Surface,o:Options)->Result<Report> {build(a,b,o,Order::C1)}
/// Quintic bridge additionally matching second transverse jets. The complete
/// U-dependent jet fields imply C2 under the explicit affine correspondence
/// in real arithmetic, hence G2 at regular boundary charts.
pub fn g2(a:&Surface,b:&Surface,o:Options)->Result<Report> {build(a,b,o,Order::C2)}

/// Whole-interval qualification on the original support surfaces. A proposal
/// may be returned with unresolved joins; it is never a solid certificate.
pub struct QualifiedReport {
    pub construction:Report,
    pub support_seams:Vec<(usize,usize,[f64;2],crate::surface_g2::Report)>,
    pub internal_seams:Vec<(usize,usize,crate::surface_g2::Report)>,
    pub complete:bool,
    pub within_tolerance:bool,
    pub cells:usize,
}
pub fn g2_qualified(a:&Surface,b:&Surface,o:Options,tolerances:[f64;3],max_cells:usize)->Result<QualifiedReport> {
    use crate::{surface_g2,surface_join::{Boundary as Edge,Decision}};
    check(max_cells<=100000,"G2 blend qualification cell limit exceeded")?;
    check(tolerances.iter().all(|t|t.is_finite()&&*t>=0.)&&tolerances[1]<=2.,"Invalid G2 blend tolerances")?;
    let construction=g2(a,b,o)?;
    let sources=[canonical(a,o.boundaries[0])?,canonical(b,o.boundaries[1])?];
    let mut result=QualifiedReport{construction,support_seams:Vec::new(),internal_seams:Vec::new(),complete:true,within_tolerance:true,cells:0};
    for (i,patch) in result.construction.patches.iter().enumerate() {
        let range=[patch.knots_u[patch.degree_u],patch.knots_u[patch.control_points.len()]];
        for side in 0..2 {
            if result.cells==max_cells {result.complete=false;result.within_tolerance=false;continue;}
            let r=surface_g2::inspect_boundary_ranges(patch,if side==0{Edge::VMin}else{Edge::VMax},
                &sources[side],if o.boundaries[side].end(){Edge::VMax}else{Edge::VMin},o.reverse_u[side],
                &[[0.,range[0]],[1.,range[1]]],tolerances[0],tolerances[1],tolerances[2],max_cells-result.cells)?;
            result.cells+=r.cells;result.complete&=r.decision!=Decision::Unresolved;result.within_tolerance&=r.decision==Decision::WithinTolerance;
            result.support_seams.push((i,side,range,r));
        }
        if i>0 {
            if result.cells==max_cells {result.complete=false;result.within_tolerance=false;continue;}
            let r=surface_g2::inspect(&result.construction.patches[i-1],Edge::UMax,patch,Edge::UMin,false,
                tolerances[0],tolerances[1],tolerances[2],max_cells-result.cells)?;
            result.cells+=r.cells;result.complete&=r.decision!=Decision::Unresolved;result.within_tolerance&=r.decision==Decision::WithinTolerance;
            result.internal_seams.push((i-1,i,r));
        }
    }
    Ok(result)
}
