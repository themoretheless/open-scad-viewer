//! Uniform periodic cubic profile transport with exact closing strip jets.
//! Dyadic station knots give a C2 basis throughout V, including the seam.
//! A separate whole-surface certificate covers interpolation and quantization.
use crate::{Result,check,continuity,curve::Curve,surface::Surface};
use value_codec::{Value,json};

/// Exact uniform cubic B-spline to Bernstein conversion. Quantized controls
/// are multiples of six dyadic units; all sums fit within 50 significant bits.
/// Consequently the divisions by three and six below have exact dyadic results.
fn patch(s:&Surface,last:bool)->Result<Surface> {
    let n=s.control_points[0].len();
    let start=if last {n-4} else {0};
    let mut out=s.clone();out.periodic_v=false;
    out.knots_v=vec![0.,0.,0.,0.,1.,1.,1.,1.];
    for row in &mut out.control_points {
        let q=&row[start..start+4];
        let mut b=vec![vec![0.;3];4];
        for k in 0..3 {
            b[0][k]=(q[0][k]+4.*q[1][k]+q[2][k])/6.;
            b[1][k]=(2.*q[1][k]+q[2][k])/3.;
            b[2][k]=(q[1][k]+2.*q[2][k])/3.;
            b[3][k]=(q[1][k]+4.*q[2][k]+q[3][k])/6.;
        }
        *row=b;
    }
    for row in &mut out.weights {*row=vec![row[0];4];}
    out.validate()?;Ok(out)
}
pub(super) fn build(sections:&[Curve])->Result<(Surface,Value)> {
    let n=sections.len()-1;
    check(n>=4 && n.is_power_of_two(),"Smooth closing stations require a dyadic period")?;
    // Uniform cyclic interpolation: Q(i-1) + 4Q(i) + Q(i+1) = 6P(i).
    let mut rows=Vec::new();
    for index in 0..sections[0].control_points.len() {
        let mut matrix:Vec<Vec<f64>>=vec![vec![0.;n];n];
        let mut rhs=vec![[0.;3];n];
        for i in 0..n {
            matrix[i][i]=4.;matrix[i][(i+n-1)%n]=1.;matrix[i][(i+1)%n]=1.;
            for k in 0..3 {rhs[i][k]=6.*sections[i].control_points[index][k];}
        }
        for i in 0..n {
            let pivot=matrix[i][i];
            check(pivot.is_finite() && pivot>0.,"Cyclic interpolation pivot unresolved")?;
            for j in i+1..n {
                let factor=matrix[j][i]/pivot;
                for k in i+1..n {matrix[j][k]-=factor*matrix[i][k];}
                for k in 0..3 {rhs[j][k]-=factor*rhs[i][k];}
            }
        }
        let mut q=vec![[0.;3];n];
        for i in (0..n).rev() {for k in 0..3 {
            let sum=(i+1..n).map(|j|matrix[i][j]*q[j][k]).sum::<f64>();
            q[i][k]=(rhs[i][k]-sum)/matrix[i][i];
        }}
        let magnitude=q.iter().flatten().fold(0_f64,|m,x|m.max(x.abs()));
        check(magnitude.is_finite(),"Cyclic interpolation coefficients overflow")?;
        let exponent=if magnitude==0. {-1022} else {((magnitude.to_bits()>>52)&0x7ff) as i32-1023};
        let power=(exponent-42).max(-1074);
        let step=if power>=-1022 {f64::from_bits(((power+1023) as u64)<<52)}
            else {f64::from_bits(1u64<<((power+1074) as u32))};
        for p in &mut q {for x in p {
            *x=(*x/step/6.).round()*6.*step;
        }}
        let mut row:Vec<Vec<f64>>=(0..n).map(|i|q[(i+n-1)%n].to_vec()).collect();
        let repeated=row[..3].to_vec();row.extend(repeated);rows.push(row);
    }
    let count=n+3;
    let surface=Surface {degree_u:sections[0].degree,degree_v:3,
        knots_u:sections[0].knots.clone(),knots_v:(0..n+7).map(|i|(i as f64-3.)/n as f64).collect(),
        control_points:rows,weights:sections[0].weights.iter().map(|w|vec![*w;count]).collect(),
        periodic_u:sections[0].periodic,periodic_v:true};
    surface.validate()?;
    let a=patch(&surface,false)?;let b=patch(&surface,true)?;
    let mut order=0;let mut work=0;let mut reason="exact-closing-jets-unproved";
    for requested in [2,1] {
        let r=continuity::inspect_surface_exact_strip_jets(&b,&a,"vMax","vMin",requested,1.,1000000-work)?;
        work+=r.work;reason=r.reason;
        if r.certified {order=requested;break;}
    }
    Ok((surface,json!({"certified":order>0,"order":order,"exact":order>0,
        "regularityCertified":order>0,"work":work,"reason":reason,
        "scope":"represented-closing-strip-jets","method":"dyadic-cubic-closing-strips-exact-predicate",
        "vBasisContinuity":"C2","stationInterpolation":"cyclic-uniform-cubic"})))
}
