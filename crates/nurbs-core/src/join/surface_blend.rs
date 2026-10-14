//! Rational homogeneous Hermite bridges between arbitrary surface V boundaries.
//! Matches source parametric jets in real arithmetic under an explicit affine
//! U correspondence. Rounding, regularity and global injectivity are separate.
use crate::{Result,check,numeric,curve::Curve,surface::Surface};
#[derive(Clone,Copy,Debug)]
pub struct Options {
    pub a_end:bool, pub b_end:bool,
    /// Positive source-V units per blend-V unit. Boundary orientation supplies sign.
    pub gains:[f64;2], pub reverse_u:[bool;2], pub max_patches:usize,
}
impl Default for Options {fn default()->Self {Self {a_end:true,b_end:false,gains:[1.;2],reverse_u:[false;2],max_patches:256}}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Order { C1, C2 }
#[derive(Clone,Debug)]
pub struct Report {
    pub patches:Vec<Surface>,
    /// Requested homogeneous jet matching order in exact real arithmetic.
    /// This is not a rounding-inclusive continuity or regularity certificate.
    pub real_arithmetic_order:Order,
}
/// Pack homogeneous boundary jets into polynomial XYZ curves: [Hx,Hy,Hw]
/// and [Hz,0,0]. Their unit weights allow exact-real basis alignment without
/// treating Cartesian vector fields with different rational denominators as
/// if they shared the boundary's denominator.
fn fields(s:&Surface,end:bool,gain:f64,reverse:bool,order:usize)->Result<Vec<Curve>> {
    s.validate()?;
    let nv=s.control_points[0].len();
    let v=if end {s.knots_v[nv]} else {s.knots_v[s.degree_v]};
    let basis=crate::curve::basis::basis(s.degree_v,&s.knots_v,nv,v,s.periodic_v)?;
    let derivatives=[basis.basis,basis.d1,basis.d2];
    let scale=s.weights.iter().flatten().copied().fold(0_f64,f64::max);
    let mut result=Vec::new();
    for k in 0..=order {
        let factor=if k==0 {1.} else if k==1 {gain} else {gain*gain};
        numeric(factor.is_finite(),"Blend transverse scale overflow")?;
        let mut h=Vec::new();
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
    let q=if order==Order::C1 {1} else {2};
    let mut input=fields(a,o.a_end,if o.a_end {o.gains[0]} else {-o.gains[0]},o.reverse_u[0],q)?;
    input.extend(fields(b,o.b_end,if o.b_end {-o.gains[1]} else {o.gains[1]},o.reverse_u[1],q)?);
    let aligned=crate::sections::compatible(&input)?;
    let pieces=aligned.iter().map(Curve::decompose).collect::<Result<Vec<_>>>()?;
    let count=pieces[0].len();
    check(count<=o.max_patches,"Blend patch budget exceeded")?;
    check(pieces.iter().all(|p|p.len()==count),"Blend aligned fields have inconsistent span counts")?;
    let vdegree=if q==1 {3} else {5};let mut patches=Vec::new();
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
        let mut controls=Vec::new();let mut weights=Vec::new();
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
        let s=Surface {degree_u:base.degree,degree_v:vdegree,knots_u:base.knots.clone(),knots_v:std::iter::repeat_n(0.,vdegree+1).chain(std::iter::repeat_n(1.,vdegree+1)).collect(),control_points:controls,weights,periodic_u:false,periodic_v:false};
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
