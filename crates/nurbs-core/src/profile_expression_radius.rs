//! Complete radius-law coverage of the exact corrected Hermite expression.
//! Reports carry no B-rep admission or caller-created certificate authority.
use crate::{check,Result,curve::Curve,interval_eval::{self,Interval as I},
    profile_section_bridge::RailConstrainedBridge};
pub struct Report {
    pub complete:bool,pub cells:usize,pub spans:usize,pub accepted_cells:usize,
    pub accepted_error_upper:f64,pub pending:Vec<[[f64;2];2]>,pub reason:&'static str,
    pub violation:Option<Violation>,
}
pub struct Violation {
    pub domain:[[f64;2];2],pub error_bounds:[f64;2],pub squared_residual:[f64;2],
}
fn mapped(d:[f64;2],across:[f64;2])->Result<I> {
    I::point(d[0]).add(I::new(across[0],across[1])?.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?.intersect(d[0],d[1])
}
fn spans(curve:&Curve,t:I)->usize {
    (curve.degree..curve.control_points.len()).filter(|&i|curve.knots[i]<curve.knots[i+1]
        && curve.knots[i]<=t.hi && curve.knots[i+1]>=t.lo).count()
}
impl RailConstrainedBridge<'_,'_,'_,'_,'_> {
    /// Center and radius curves are mapped over their complete domains by
    /// normalized across coordinate. Radius uses coordinate zero. All free
    /// coordinates participate; no sample or unchanged source admits a cell.
    pub fn inspect_radius(&self,centers:&Curve,radius:&Curve,tolerance:f64,
        max_cells:usize,max_spans:usize)->Result<Report> {
        centers.validate()?;radius.validate()?;
        self.inspect_radius_on_domains(centers,radius,[centers.domain(),radius.domain()],tolerance,max_cells,max_spans)
    }
    /// Restrict the original law parameters without rebuilding either curve.
    /// Each ordered domain maps affinely from the entire across interval.
    pub fn inspect_radius_on_domains(&self,centers:&Curve,radius:&Curve,domains:[[f64;2];2],tolerance:f64,
        max_cells:usize,max_spans:usize)->Result<Report> {
        centers.validate()?;radius.validate()?;
        for (curve,domain) in [centers,radius].into_iter().zip(domains) {
            let full=curve.domain();
            check(domain.iter().all(|x|x.is_finite()) && full[0]<=domain[0] && domain[0]<domain[1]
                && domain[1]<=full[1],"Keep original radius-law domain ordered inside its curve")?;
        }
        check(centers.control_points[0].len()==3 && radius.control_points[0].len()==2
            && radius.control_points.iter().all(|p|p[0]>=0.) && tolerance.is_finite() && tolerance>0.
            && max_cells<=100000 && max_spans<=1000000,"Bound exact expression radius law and work")?;
        let range=self.section(false).range();
        let mut out=Report{complete:false,cells:0,spans:0,accepted_cells:0,accepted_error_upper:0.,
            pending:vec![[range,[0.,1.]]],reason:"expression-radius-work-limit",violation:None};
        while let Some(rect)=out.pending.pop() {
            if out.cells==max_cells || out.spans==max_spans {out.pending.push(rect);return Ok(out);}
            out.cells+=1;
            let (position,used)=self.enclose_with_transverse_correction(rect[0],rect[1],(max_spans-out.spans).min(100000))?;
            out.spans+=used;
            let Some(position)=position else {out.pending.push(rect);return Ok(out);};
            let ct=mapped(domains[0],rect[1])?;let rt=mapped(domains[1],rect[1])?;
            let used=spans(centers,ct)+spans(radius,rt);
            if used>max_spans-out.spans {out.pending.push(rect);return Ok(out);}
            out.spans+=used;
            let c=interval_eval::evaluate_interval(centers,ct)?;
            let r=interval_eval::evaluate_interval(radius,rt)?;
            let bound=crate::moving_radius::bound_expression(position,
                std::array::from_fn(|axis|[c[axis].lo,c[axis].hi]),[r[0].lo.max(0.),r[0].hi])?;
            // A rigorously enclosed point may refute the law, never admit the
            // rectangle. Probe the initial center before depth-first coverage
            // spends all work near a boundary with zero residual.
            if out.cells==1 && bound.error_upper>tolerance && max_spans-out.spans>=34 {
                let point=rect.map(|d|[d[0]*0.5+d[1]*0.5;2]);
                let (p,used)=self.enclose_with_transverse_correction(point[0],point[1],(max_spans-out.spans).min(100000))?;
                out.spans+=used;
                if let Some(p)=p {
                    let ct=mapped(domains[0],point[1])?;let rt=mapped(domains[1],point[1])?;
                    let used=spans(centers,ct)+spans(radius,rt);
                    if used<=max_spans-out.spans {
                        out.spans+=used;
                        let c=interval_eval::evaluate_interval(centers,ct)?;let r=interval_eval::evaluate_interval(radius,rt)?;
                        let probe=crate::moving_radius::bound_expression(p,
                            std::array::from_fn(|axis|[c[axis].lo,c[axis].hi]),[r[0].lo.max(0.),r[0].hi])?;
                        if probe.error_lower>tolerance {
                            out.violation=Some(Violation{domain:point,error_bounds:[probe.error_lower,probe.error_upper],squared_residual:probe.squared_residual});
                            out.pending.push(rect);out.reason="expression-radius-law-mismatch";return Ok(out);
                        }
                    }
                }
            }
            if bound.error_upper<=tolerance {
                out.accepted_cells+=1;out.accepted_error_upper=out.accepted_error_upper.max(bound.error_upper);continue;
            }
            if bound.error_lower>tolerance {
                out.violation=Some(Violation{domain:rect,error_bounds:[bound.error_lower,bound.error_upper],
                    squared_residual:bound.squared_residual});
                out.pending.push(rect);out.reason="expression-radius-law-mismatch";return Ok(out);
            }
            let axis=usize::from(rect[1][1]-rect[1][0]>(rect[0][1]-rect[0][0])/(range[1]-range[0]));
            let mid=rect[axis][0]*0.5+rect[axis][1]*0.5;
            if mid<=rect[axis][0] || mid>=rect[axis][1] {
                out.pending.push(rect);out.reason="expression-radius-resolution-limit";return Ok(out);
            }
            let mut left=rect;left[axis][1]=mid;let mut right=rect;right[axis][0]=mid;
            out.pending.push(right);out.pending.push(left);
        }
        out.complete=true;out.reason="expression-radius-complete-coverage";Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_cylinder_radius_covers_whole_corrected_expression() {
        let w=std::f64::consts::FRAC_1_SQRT_2;
        let surface=crate::surface::Surface{degree_u:1,degree_v:2,
            knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:[0.,1.].into_iter().map(|x|vec![vec![x,1.,0.],vec![x,1.,1.],vec![x,0.,1.]]).collect(),
            weights:vec![vec![1.,w,1.];2],periodic_u:false,periodic_v:false};
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
        let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let centers=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.]],
            weights:vec![1.;2],periodic:false};
        let radius=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![1.,0.],vec![1.,0.]],
            weights:vec![1.;2],periodic:false};
        let report=corrected.inspect_radius(&centers,&radius,0.1,10000,1000000).unwrap();
        assert!(report.complete,"{} cells={} spans={} pending={}",report.reason,report.cells,report.spans,report.pending.len());
        assert!(report.pending.is_empty());assert!(report.accepted_error_upper<=0.1);
        let mut extended=centers.clone();extended.control_points[0][0]=-1.;extended.control_points[1][0]=2.;
        let restricted=corrected.inspect_radius_on_domains(&extended,&radius,[[1./3.,2./3.],[0.25,0.75]],0.1,10000,1000000).unwrap();
        assert!(restricted.complete);assert!(restricted.pending.is_empty());
        assert!(corrected.inspect_radius_on_domains(&centers,&radius,[[-0.1,1.],[0.,1.]],0.1,100,10000).is_err());
        assert!(corrected.inspect_radius_on_domains(&centers,&radius,[[0.5,0.5],[0.,1.]],0.1,100,10000).is_err());
        for (cells,spans) in [(0,1000),(100,0),(1,1)] {
            let refused=corrected.inspect_radius(&centers,&radius,0.1,cells,spans).unwrap();
            assert!(!refused.complete);assert!(!refused.pending.is_empty());
        }
        let mut wrong=radius.clone();for p in &mut wrong.control_points {p[0]=2.;}
        let refused=corrected.inspect_radius(&centers,&wrong,0.1,100,10000).unwrap();
        assert!(!refused.complete);assert!(!refused.pending.is_empty());
        assert_eq!(refused.reason,"expression-radius-law-mismatch");
        let violation=refused.violation.unwrap();
        assert!(violation.error_bounds[0]>0.1);
        assert!(violation.error_bounds[0]<=1. && violation.error_bounds[1]>=1.);
    }
}
