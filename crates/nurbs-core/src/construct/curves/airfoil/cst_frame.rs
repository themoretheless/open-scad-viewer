//! Authored orthogonal CST coordinates; no exact-geometry certificate.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct CstFrame {
    pub origin: [f64; 3],
    /// Directed physical chord vector (length and orientation).
    pub chord: [f64; 3],
    /// Positive thickness direction, orthogonal to the chord.
    pub ordinate_direction: [f64; 3],
}
fn dot(a:[f64;3],b:[f64;3])->f64 {(0..3).map(|i|a[i]*b[i]).sum()}
fn cross(a:[f64;3],b:[f64;3])->[f64;3] {std::array::from_fn(|i|a[(i+1)%3]*b[(i+2)%3]-a[(i+2)%3]*b[(i+1)%3])}
fn length(a:[f64;3])->f64 {a[0].hypot(a[1]).hypot(a[2])}
fn unit(a:[f64;3])->Result<[f64;3]> {
    check(a.iter().all(|x|x.is_finite()),"CST axis must be finite")?;
    let scale=a.iter().fold(0_f64,|s,x|s.max(x.abs()));
    check(scale>0.,"CST axis must be nonzero")?;
    let b=a.map(|x|x/scale);let n=length(b);Ok(b.map(|x|x/n))
}
impl CstFrame {
    pub fn new(origin:[f64;3],chord:[f64;3],ordinate_direction:[f64;3])->Result<Self> {
        let frame=Self{origin,chord,ordinate_direction:unit(ordinate_direction)?};
        frame.axes()?;Ok(frame)
    }
    pub(super) fn axes(&self)->Result<([f64;3],[f64;3],[f64;3],f64)> {
        check(self.origin.iter().all(|x|x.is_finite()),"CST frame origin must be finite")?;
        let x=unit(self.chord)?;let y=unit(self.ordinate_direction)?;
        check(dot(x,y).abs()<=64.*f64::EPSILON,"CST ordinate must be orthogonal to the chord")?;
        let chord=length(self.chord);
        crate::numeric(chord.is_finite()&&chord>0.,"CST physical chord is not representable")?;
        Ok((x,y,unit(cross(x,y))?,chord))
    }
    /// Map dimensionless chord/ordinate coordinates to the authored plane.
    pub fn point(&self,u:f64,v:f64)->Result<[f64;3]> {
        check(u.is_finite()&&v.is_finite(),"CST coordinates must be finite")?;
        let (_,y,_,chord)=self.axes()?;
        let p=std::array::from_fn(|i|self.origin[i]+self.chord[i]*u+y[i]*(chord*v));
        crate::numeric(p.iter().all(|x|x.is_finite()),"CST world placement overflowed")?;Ok(p)
    }
    pub(super) fn inferred(points:&[[f64;3]])->Result<Self> {
        let xmin=points.iter().map(|p|p[0]).fold(f64::INFINITY,f64::min);
        let xmax=points.iter().map(|p|p[0]).fold(f64::NEG_INFINITY,f64::max);
        check(xmin<xmax,"CST data degenerate in the X chord direction; supply an authored frame")?;
        let mean=|x:f64|-> [f64;3] {
            let edge:Vec<_>=points.iter().filter(|p|p[0]==x).collect();
            std::array::from_fn(|i| {
                if i==0 {return x;}
                let scale=edge.iter().fold(0_f64,|m,p|m.max(p[i].abs()));
                if scale==0. {0.} else {edge.iter().map(|p|p[i]/scale).sum::<f64>()/edge.len() as f64*scale}
            })
        };
        let origin=mean(xmin);let end=mean(xmax);let chord=std::array::from_fn(|i|end[i]-origin[i]);
        let x=unit(chord)?;let normal=unit(cross(x,[0.,1.,0.]))?;
        Self::new(origin,chord,cross(normal,x))
    }
}
impl CstFitResult {
    /// Bounded checked sampling in the recovered world frame.
    pub fn points(&self,count:usize)->Result<Vec<[f64;3]>> {
        cst_points(&self.profile,1.,count)?.iter().map(|p|self.frame.point(p[0],p[1])).collect()
    }
    /// Numerical NURBS fit in normalized coordinates, placed in the world frame.
    /// No continuous approximation or regularity certificate is returned.
    pub fn curve(&self,samples:usize,degree:usize,controls:usize)->Result<Curve> {
        let mut c=cst_airfoil(&self.profile,1.,samples,degree,controls)?;
        for p in &mut c.control_points {*p=self.frame.point(p[0],p[1])?.to_vec();}
        c.validate()?;Ok(c)
    }
}

/// Fit a CST profile to checked analytic samples as a planar NURBS.
/// Control/sample budgets are identical to the existing airfoil fitter.
pub fn cst_airfoil(profile:&CstProfile,chord:f64,samples:usize,degree:usize,controls:usize)->Result<Curve> {
    let points=cst_points(profile,chord,samples)?;
    fit_open_bspline(&points,degree,controls,0.,Some(true))
}
