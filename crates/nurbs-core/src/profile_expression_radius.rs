//! Complete radius-law coverage of the exact corrected Hermite expression.
//! Reports carry no B-rep admission or caller-created certificate authority.
use crate::{
    check,
    curve::Curve,
    interval_eval::{self, Interval as I},
    profile_section_bridge::RailConstrainedBridge,
    Result,
};
pub use crate::profile_expression_bernstein::CoefficientRadiusBound;
pub use crate::profile_expression_bernstein::CoefficientLineReport;
pub struct Report {
    pub domain: [[f64; 2]; 2],
    pub complete: bool,
    pub cells: usize,
    pub spans: usize,
    pub accepted_cells: usize,
    pub accepted_error_upper: f64,
    pub pending: Vec<[[f64; 2]; 2]>,
    pub reason: &'static str,
    pub violation: Option<Violation>,
}
impl Report {
    /// Diagnostic parameter-area fraction, not physical surface area or proof.
    /// Admission continues to require complete coverage with no pending cells.
    pub fn pending_parameter_fraction(&self) -> f64 {
        self.pending
            .iter()
            .map(|rect| {
                (0..2)
                    .map(|axis| {
                        (rect[axis][1] - rect[axis][0])
                            / (self.domain[axis][1] - self.domain[axis][0])
                    })
                    .product::<f64>()
            })
            .sum::<f64>()
            .clamp(0., 1.)
    }
}
pub struct Violation {
    pub domain: [[f64; 2]; 2],
    pub error_bounds: [f64; 2],
    pub squared_residual: [f64; 2],
}
fn mapped(d: [f64; 2], across: [f64; 2]) -> Result<I> {
    I::point(d[0])
        .add(I::new(across[0], across[1])?.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
        .intersect(d[0], d[1])
}
fn spans(curve: &Curve, t: I) -> usize {
    (curve.degree..curve.control_points.len())
        .filter(|&i| {
            curve.knots[i] < curve.knots[i + 1]
                && curve.knots[i] <= t.hi
                && curve.knots[i + 1] >= t.lo
        })
        .count()
}
fn continuous_knots(knots: &[f64], degree: usize, count: usize, domain: [f64; 2]) -> bool {
    !knots[degree + 1..count].iter().any(|&knot| {
        domain[0] <= knot
            && knot <= domain[1]
            && knots.iter().filter(|&&k| k == knot).count() > degree
    })
}
impl crate::surface_weight_profile::WeightProfile<'_> {
    /// Endpoint-specific bound on one continuous exact profile chart.
    pub fn inspect_self_chord_pair(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,
    ) -> Result<(Option<bool>,usize)> {
        self.inspect_self_chord_pair_cached(pair,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
    fn inspect_self_chord_pair_cached(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,
        cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>,usize)> {
        check(max_spans<=100000 && pair.iter().all(|uv|uv.iter().all(|d|
            d.iter().all(|x|x.is_finite()) && d[0]<d[1])),"Bound profile chord endpoints and work")?;
        let union=std::array::from_fn(|k|[pair[0][k][0].min(pair[1][k][0]),pair[0][k][1].max(pair[1][k][1])]);
        self.inspect_self_chords(union,max_sine_squared,0)?;
        let source=self.source();
        for (knots,degree,count,domain) in [(&source.knots_u,source.degree_u,source.control_points.len(),union[0]),
            (&source.knots_v,source.degree_v,source.control_points[0].len(),union[1])] {
            if degree==0 || !continuous_knots(knots,degree,count,domain) {return Ok((None,0));}
        }
        if max_spans==0 {return Ok((None,0));}
        let (jacobian,mut spent)=cache.enclose(self,union,max_spans)?;
        let Some(mut jacobian)=jacobian else {return Ok((None,spent));};
        if !crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)? {
            // Connect A to B through (B.u,A.v): U varies only on A's
            // V strip, then V only on B's U strip. Continuity above covers
            // both paths. Each integrated column bounds its own full strip.
            let strips=[[union[0],pair[0][1]],[pair[1][0],union[1]]];
            let mut columns=jacobian;
            for axis in 0..2 {
                if spent==max_spans {return Ok((None,spent));}
                let (bounds,used)=cache.enclose(self,strips[axis],max_spans-spent)?;
                spent+=used;
                let Some(bounds)=bounds else {return Ok((None,spent));};
                for k in 0..3 {columns[k][axis]=bounds[k][axis];}
            }
            jacobian=columns;
        }
        // Direct M*(B-A) interval exclusion proves its own nonzero chord;
        // only the mean-matrix theorem requires uniform rank admission.
        let ranked=crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)?;
        let mut normals=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            // Only chord/normal angles are used here; strictly interior pole
            // factors preserve direction without changing Jacobian magnitudes.
            let (normal,used)=cache.profile_direction(self,endpoint,max_spans-spent)?;
            spent+=used;
            let Some(normal)=normal else {return Ok((None,spent));};
            normals[index]=normal;
            if crate::surface_self_chord::inspect_jacobian_endpoint_pair_chords(jacobian,pair,normal,max_sine_squared)?
                || (ranked && crate::surface_self_chord::inspect_jacobian_endpoint_chords(jacobian,normal,max_sine_squared)?.0) {
                return Ok((Some(true),spent));
            }
        }
        // Both coordinate paths bound the same exact profile chord, even
        // when the wider union already has full rank. Only complete paths
        // participate in this independent chord-box intersection.
        let mut chord=[[f64::NEG_INFINITY,f64::INFINITY];3];
        let mut delta=[I::point(0.);2];
        for axis in 0..2 {
            delta[axis]=I::new(pair[1][axis][0],pair[1][axis][1])?
                .sub(I::new(pair[0][axis][0],pair[0][axis][1])?)?;
        }
        for reversed in [false,true] {
            let strips=if reversed {[[union[0],pair[1][1]],[pair[0][0],union[1]]]}
                else {[[union[0],pair[0][1]],[pair[1][0],union[1]]]};
            let mut columns=jacobian;
            for axis in 0..2 {
                if spent==max_spans {return Ok((None,spent));}
                let (jet,used)=cache.enclose(self,strips[axis],max_spans-spent)?;spent+=used;
                let Some(jet)=jet else {return Ok((None,spent));};
                for k in 0..3 {columns[k][axis]=jet[k][axis];}
            }
            for k in 0..3 {
                let value=I::new(columns[k][0][0],columns[k][0][1])?.mul(delta[0])?
                    .add(I::new(columns[k][1][0],columns[k][1][1])?.mul(delta[1])?)?;
                chord[k]=[chord[k][0].max(value.lo),chord[k][1].min(value.hi)];
                I::new(chord[k][0],chord[k][1])?;
            }
            // The first complete path is sufficient when its chord box
            // already excludes alignment. Otherwise the reverse path tightens
            // the same exact chord by interval intersection as before.
            for normal in normals {
                if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                    return Ok((Some(true),spent));
                }
            }
        }
        let mut positions=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            let (position,used)=cache.position(self,endpoint,max_spans-spent)?;spent+=used;
            let Some(position)=position else {return Ok((None,spent));};positions[index]=position;
        }
        for k in 0..3 {
            let value=I::new(positions[1][k][0],positions[1][k][1])?
                .sub(I::new(positions[0][k][0],positions[0][k][1])?)?;
            let overlap=I::new(chord[k][0],chord[k][1])?.intersect(value.lo,value.hi)?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                return Ok((Some(true),spent));
            }
        }
        Ok((Some(false),spent))
    }
    /// Inspect normal-aligned self chords of the exact separable-weight profile.
    /// Positive recipe weights preserve chart continuity; derivative bounds are
    /// recomputed from this profile rather than inherited from its source.
    pub fn inspect_self_chords(
        &self, domain: [[f64; 2]; 2], max_sine_squared: f64, max_spans: usize,
    ) -> Result<(Option<bool>, usize)> {
        self.inspect_self_chords_cached(domain,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
    fn inspect_self_chords_cached(
        &self,domain:[[f64;2];2],max_sine_squared:f64,max_spans:usize,
        cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>,usize)> {
        let source = self.source();
        let knots = [&source.knots_u, &source.knots_v];
        let degrees = [source.degree_u, source.degree_v];
        let counts = [source.control_points.len(), source.control_points[0].len()];
        check(max_spans <= 100000 && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && (0..2).all(|axis| domain[axis].iter().all(|x| x.is_finite())
                && domain[axis][0] < domain[axis][1]
                && domain[axis][0] >= knots[axis][degrees[axis]]
                && domain[axis][1] <= knots[axis][counts[axis]]),
            "Bound profile self-chord domain, angle and work")?;
        if (0..2).any(|axis| degrees[axis] == 0
            || !continuous_knots(knots[axis], degrees[axis], counts[axis], domain[axis]))
            || max_spans == 0 {
            return Ok((None, 0));
        }
        let (jacobian, spent) = cache.enclose(self,domain,max_spans)?;
        let Some(jacobian) = jacobian else { return Ok((None, spent)); };
        let (excluded, _) = crate::surface_self_chord::inspect_jacobian_self_chords(
            jacobian, max_sine_squared)?;
        Ok((Some(excluded), spent))
    }
}
#[cfg(test)]
mod profile_self_chord_tests {
    use super::*;
    #[test]
    fn coordinate_strips_exclude_planar_chart_with_collapsed_boundary() {
        let s=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,0.,0.]],
                vec![vec![0.,1.,0.],vec![1.,0.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
        };
        let p=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
        let full=p.derivative_bounds([[0.,1.];2],100).unwrap().0.unwrap();
        assert!(!crate::surface_self_chord::inspect_jacobian_rank_bound(full).unwrap());
        let pair=[[[0.,0.1],[0.,0.1]],[[0.9,1.],[0.9,1.]]];
        assert_eq!(p.inspect_self_chord_pair(pair,1e-4,100).unwrap().0,Some(true));
        assert_eq!(p.inspect_self_chord_pair(pair,1e-4,1).unwrap(),(None,1));
        // S(u,v)=(uv,u(1-v),0): every nonzero chord lies in XY;
        // at B, u>=0.9, the analytic cross product is (0,0,-u).
        // The collapsed A boundary is not assigned a regular normal.
        let mut spatial=s.clone();
        for row in &mut spatial.control_points {
            for (v,point) in row.iter_mut().enumerate() {point[2]=v as f64;}
        }
        let spatial=crate::surface_weight_profile::qualify_weight_profile(&spatial,[0,0],100).unwrap().0.unwrap();
        // The endpoint-domain enclosure resolves the previously insufficient
        // S=(uv,u(1-v),v) control. Analytic corner chords check the direction.
        assert_eq!(spatial.inspect_self_chord_pair(pair,1e-4,100).unwrap().0,Some(true));
        for u in [0.,0.1] {for v in [0.,0.1] {for x in [0.9,1.] {for y in [0.9,1.] {
            let chord=[x*y-u*v,x*(1.-y)-u*(1.-v),y-v];
            for (a,b) in [(u,v),(x,y)] {
                let normal=[1.-b,-b,-a];
                let dot:f64=(0..3).map(|k|normal[k]*chord[k]).sum();
                let nn:f64=normal.iter().map(|n|n*n).sum();
                let cc:f64=chord.iter().map(|c|c*c).sum();
                assert!(dot*dot/(nn*cc)<1.-1e-4);
            }
        }}}}
    }
    #[test]
    fn rational_interior_endpoint_directions_keep_planar_chords_excluded() {
        let mut s=crate::surface::Surface {
            degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],
                vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.,4.];3],periodic_u:false,periodic_v:false,
        };
        let pair=[[[0.3,0.31],[0.49,0.51]],[[0.5,0.51],[0.49,0.51]]];
        for reversed in [false,true] {
            if reversed {s.control_points.reverse();}
            let p=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
            let (excluded,spent)=p.inspect_self_chord_pair(pair,1e-4,100).unwrap();
            assert_eq!(excluded,Some(true));assert!(spent<=100);
            assert_eq!(p.inspect_self_chord_pair(pair,1e-4,0).unwrap(),(None,0));
            let wide=[[[0.01,0.05],[0.49,0.51]],[[0.8,0.9],[0.49,0.51]]];
            assert_eq!(p.inspect_self_chord_pair(wide,1e-4,100).unwrap().0,Some(true),
                "direct chord exclusion does not require uniform matrix rank");
            // Both endpoint charts have nonzero normal +/-Z and every chord
            // lies in XY. Positive interior factor removal preserves this angle.
        }
    }
    #[test]
    fn retained_workspace_reuses_complete_hulls_and_preserves_refusals() {
        let s=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
                vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
        let pair=[[[0.,0.2],[0.,0.2]],[[0.8,1.],[0.8,1.]]];
        let direct=profile.inspect_self_chord_pair(pair,1e-4,100).unwrap();
        let mut workspace=profile.chord_workspace();
        assert_eq!(workspace.inspect_pair(pair,1e-4,100).unwrap(),direct);
        let warm=workspace.inspect_pair(pair,1e-4,100).unwrap();
        assert_eq!(warm.0,direct.0);assert!(warm.1<direct.1);
        assert!(workspace.cache_hits()>0);
        let union=std::array::from_fn(|axis|[pair[0][axis][0].min(pair[1][axis][0]),pair[0][axis][1].max(pair[1][axis][1])]);
        let intrinsic=profile.inspect_self_chords(union,1e-4,100).unwrap();
        let shared=workspace.inspect_self_chords(union,1e-4,100).unwrap();
        assert_eq!(shared.0,intrinsic.0);
        assert!(shared.1<intrinsic.1);
        assert_eq!(workspace.inspect_self_chords(union,1e-4,0).unwrap(),(None,0));
        assert!(workspace.inspect_self_chords([[-1.,1.],[0.,1.]],1e-4,0).is_err());
        assert_eq!(workspace.inspect_pair(pair,1e-4,0).unwrap(),(None,0));
        let mut invalid=pair;invalid[0][0][1]=2.;
        assert!(workspace.inspect_pair(invalid,1e-4,0).is_err());
        assert!(workspace.inspect_pair(pair,1.,100).is_err());
        let mut retry=profile.chord_workspace();
        assert_eq!(retry.inspect_pair(pair,1e-4,1).unwrap(),(None,1));
        let completed=retry.inspect_pair(pair,1e-4,100).unwrap();
        assert_eq!(completed.0,direct.0);assert!(completed.1<=direct.1);
        // Same UV coordinates on another owner start cold.
        let other=crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
        assert_eq!(other.chord_workspace().inspect_pair(pair,1e-4,100).unwrap(),direct);
    }
    #[test]
    fn retained_half_cylinder_preserves_exact_diameter_candidate() {
        let points = [[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.]];
        let w = 0.5_f64.sqrt();
        let s = crate::surface::Surface {
            degree_u:2, degree_v:1,
            knots_u:vec![0.,0.,0.,0.5,0.5,1.,1.,1.], knots_v:vec![0.,0.,1.,1.],
            control_points:points.iter().map(|p|vec![vec![p[0],p[1],0.],vec![p[0],p[1],1.]]).collect(),
            weights:[1.,w,1.,w,1.].iter().map(|w|vec![*w;2]).collect(),
            periodic_u:false,periodic_v:false,
        };
        let p = crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
        // The exact U=0/1 endpoints have an aligned diameter chord; an
        // intrinsic exclusion of the complete chart would hide a thin wall.
        assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-6,100).unwrap().0,Some(false));
        assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-6,1).unwrap(),(None,1));
        let pair=[[[0.,0.01],[0.4,0.5]],[[0.99,1.],[0.4,0.5]]];
        assert_eq!(p.inspect_self_chord_pair(pair,1e-6,100).unwrap().0,Some(false));
        let mut workspace=p.chord_workspace();
        assert_eq!(workspace.inspect_self_chords([[0.,1.];2],1e-6,100).unwrap().0,Some(false));
        for budget in [0,1,100] {
            let result=workspace.inspect_self_chords([[0.,1.];2],1e-6,budget).unwrap();
            assert_ne!(result.0,Some(true));
            assert!(result.1<=budget);
            if budget==0 {assert_eq!(result,(None,0));}
        }
        assert_eq!(workspace.inspect_pair(pair,1e-6,100).unwrap().0,Some(false));
        for endpoints in [pair,[pair[1],pair[0]]] {for budget in [0,1,4,8,100] {
            let (decision,spent)=workspace.inspect_pair(endpoints,1e-6,budget).unwrap();
            assert_ne!(decision,Some(true),"Warm retained cache must preserve a real normal diameter");
            assert!(spent<=budget);
            if budget==0 {assert_eq!((decision,spent),(None,0));}
        }}
        assert_eq!(p.inspect_self_chord_pair(pair,1e-6,1).unwrap(),(None,1));
        for endpoints in [pair,[pair[1],pair[0]]] {
            for budget in [0,1,4,8,100] {
                let (result,spent)=p.inspect_self_chord_pair(endpoints,1e-6,budget).unwrap();
                assert!(result!=Some(true) && spent<=budget,
                    "an actual normal diameter survives both paths and incomplete work");
            }
        }
    }
    #[test]
    fn retained_diameter_candidate_survives_scale_translation_and_endpoint_exchange() {
        let w=0.5_f64.sqrt();
        for scale in [1e-8,1.,1e6] {for shift in [0.,100.] {
            let points=[[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.]];
            let surface=crate::surface::Surface {
                degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,0.5,0.5,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
                control_points:points.iter().map(|p|[0.,1.].iter().map(|z|
                    vec![scale*(p[0]+shift),scale*(p[1]-shift),scale*(z+shift)]).collect()).collect(),
                weights:[1.,w,1.,w,1.].iter().map(|w|vec![*w;2]).collect(),periodic_u:false,periodic_v:false,
            };
            let profile=crate::surface_weight_profile::qualify_weight_profile(&surface,[0,0],100).unwrap().0.unwrap();
            let pair=[[[0.,0.01],[0.4,0.5]],[[0.99,1.],[0.4,0.5]]];
            for endpoints in [pair,[pair[1],pair[0]]] {for budget in [0,1,4,8,100] {
                let (decision,spent)=profile.inspect_self_chord_pair(endpoints,1e-6,budget).unwrap();
                assert_ne!(decision,Some(true),"Normal diameter excluded at scale={scale} shift={shift} budget={budget}");
                assert!(spent<=budget);
                if budget==100 {assert_eq!(decision,Some(false));}
            }}
        }}
    }
    #[test]
    fn exact_profile_excludes_plane_but_preserves_discontinuity_and_budget_refusal() {
        let mut s = crate::surface::Surface {
            degree_u: 1, degree_v: 1,
            knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
                vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights: vec![vec![1.;2];2], periodic_u:false, periodic_v:false,
        };
        let p = crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
        assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-4,0).unwrap(),(None,0));
        assert_eq!(p.inspect_self_chords([[0.,1.];2],1e-4,100).unwrap().0,Some(true));
        assert_eq!(p.inspect_self_chord_pair([[[0.,0.4],[0.,1.]],[[0.6,1.],[0.,1.]]],1e-4,100).unwrap().0,Some(true));
        assert!(p.inspect_self_chords([[-1.,1.],[0.,1.]],1e-4,0).is_err());
        s.knots_u = vec![0.,0.,0.5,0.5,1.,1.];
        s.control_points.extend(s.control_points.clone());
        s.weights = vec![vec![1.;2];4];
        assert!(!continuous_knots(&s.knots_u,1,4,[0.,1.]));
        assert!(crate::surface_weight_profile::qualify_weight_profile(&s,[0,0],100).is_err());
    }
}
// Original rational jets differentiate the restricted local [0,1] chart.
// Convert to source parameter, then to normalized bridge across parameter.
fn law_derivative(
    curve: &Curve,
    t: I,
    law_domain: [f64; 2],
    budget: usize,
) -> Result<(Option<Vec<I>>, usize)> {
    let mut spent = 0;
    let mut hull: Option<Vec<I>> = None;
    let scale = I::point(law_domain[1]).sub(I::point(law_domain[0]))?;
    for span in curve.degree..curve.control_points.len() {
        let lo = t.lo.max(curve.knots[span]);
        let hi = t.hi.min(curve.knots[span + 1]);
        if lo >= hi {
            continue;
        }
        if spent == budget {
            return Ok((None, spent));
        }
        spent += 1;
        let jet = crate::curve_jets::enclose(curve, span, [lo, hi])?;
        let width = I::point(hi).sub(I::point(lo))?;
        let derivative = jet[1]
            .iter()
            .map(|d| d.div(width)?.mul(scale))
            .collect::<Result<Vec<_>>>()?;
        if let Some(previous) = &mut hull {
            for (a, b) in previous.iter_mut().zip(derivative) {
                *a = I::new(a.lo.min(b.lo), a.hi.max(b.hi))?;
            }
        } else {
            hull = Some(derivative);
        }
    }
    Ok((hull, spent))
}
// Public workspaces retain the immutable bridge, and thus both section
// profiles, for this entire cache lifetime. Equal UV coordinates on its two
// distinct profiles do not identify equal geometry; include profile identity.
#[derive(Default)]
struct ProfileDerivativeCache {
    side_remainders: crate::profile_section_bridge::SideRemainderCache,
    entries:std::collections::HashMap<(usize,[[u64;2];2]),[[[f64;2];2];3]>,
    corrected:std::collections::HashMap<[[u64;2];2],[[[f64;2];3];2]>,
    directions:std::collections::HashMap<(usize,[[u64;2];2]),[[f64;2];3]>,
    positions:std::collections::HashMap<(usize,[[u64;2];2]),[[f64;2];3]>,
    corrected_positions:std::collections::HashMap<[[u64;2];2],[[f64;2];3]>,
    hits:usize,
}
impl ProfileDerivativeCache {
    fn enclose(&mut self,profile:&crate::surface_weight_profile::WeightProfile<'_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[[f64;2];2];3]>,usize)> {
        let key=(profile as *const _ as usize,domain.map(|d|d.map(f64::to_bits)));
        if let Some(value)=self.entries.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=profile.derivative_bounds(domain,budget)?;
        if let Some(value)=value {if self.entries.len()<100000 {self.entries.insert(key,value);}}
        Ok((value,spent))
    }
    fn corrected_jet(&mut self,bridge:&RailConstrainedBridge<'_, '_, '_, '_, '_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[[f64;2];3];2]>,usize)> {
        let key=domain.map(|d|d.map(f64::to_bits));
        if let Some(value)=self.corrected.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=bridge.derivatives_with_transverse_correction_cached(domain[0],domain[1],budget,&mut self.side_remainders)?;
        if let Some(value)=value {if self.corrected.len()<100000 {self.corrected.insert(key,value);}}
        Ok((value,spent))
    }
    fn profile_direction(&mut self,profile:&crate::surface_weight_profile::WeightProfile<'_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let key=(profile as *const _ as usize,domain.map(|d|d.map(f64::to_bits)));
        if let Some(value)=self.directions.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=profile.interior_normal_direction_bounds(domain,budget)?;
        // A zero-containing direction box can reflect an exhausted optional
        // pole-factor attempt. Do not freeze it before a larger-budget retry.
        if let Some(value)=value {if value.iter().any(|d|d[0]>0. || d[1]<0.)
            && self.directions.len()<100000 {self.directions.insert(key,value);}}
        Ok((value,spent))
    }
    fn position(&mut self,profile:&crate::surface_weight_profile::WeightProfile<'_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let key=(profile as *const _ as usize,domain.map(|d|d.map(f64::to_bits)));
        if let Some(value)=self.positions.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=profile.enclose(domain,budget)?;
        if let Some(value)=value {if self.positions.len()<100000 {self.positions.insert(key,value);}}
        Ok((value,spent))
    }
    fn corrected_position(&mut self,bridge:&RailConstrainedBridge<'_, '_, '_, '_, '_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let key=domain.map(|d|d.map(f64::to_bits));
        if let Some(value)=self.corrected_positions.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=bridge.enclose_with_transverse_correction_cached(domain[0],domain[1],budget,Some(&self.side_remainders))?;
        if let Some(value)=value {if self.corrected_positions.len()<100000 {self.corrected_positions.insert(key,value);}}
        Ok((value,spent))
    }
    fn corrected_normal(&mut self,bridge:&RailConstrainedBridge<'_, '_, '_, '_, '_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let (jet,spent)=self.corrected_jet(bridge,domain,budget)?;
        let Some([a,b])=jet else {return Ok((None,spent));};
        let mut bounds=[[0.;2];3];
        for k in 0..3 {
            let i=(k+1)%3;let j=(k+2)%3;
            let n=I::new(a[i][0],a[i][1])?.mul(I::new(b[j][0],b[j][1])?)?
                .sub(I::new(a[j][0],a[j][1])?.mul(I::new(b[i][0],b[i][1])?)?)?;
            bounds[k]=[n.lo,n.hi];
        }
        Ok((Some(bounds),spent))
    }
}
/// Invocation-local complete hull cache bound to one immutable retained profile.
/// UV keys cannot be shared with another profile; incomplete hulls are not stored.
pub struct WeightChordWorkspace<'p,'s> {
    profile:&'p crate::surface_weight_profile::WeightProfile<'s>,
    derivatives:ProfileDerivativeCache,
}
impl WeightChordWorkspace<'_, '_> {
    pub fn cache_hits(&self)->usize {self.derivatives.hits}
    /// Whole-chart exclusion shares complete jets with endpoint-pair checks.
    /// Domain validation and zero-budget refusal precede every cache lookup.
    pub fn inspect_self_chords(&mut self,domain:[[f64;2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.profile.inspect_self_chords_cached(domain,sine,max_spans,&mut self.derivatives)
    }
    pub fn inspect_pair(&mut self,pair:[[[f64;2];2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.profile.inspect_self_chord_pair_cached(pair,sine,max_spans,&mut self.derivatives)
    }
}
impl<'s> crate::surface_weight_profile::WeightProfile<'s> {
    pub fn chord_workspace(&self)->WeightChordWorkspace<'_,'s> {
        WeightChordWorkspace {profile:self,derivatives:ProfileDerivativeCache::default()}
    }
}
/// Invocation-local cache bound to an immutable native corrected transition.
/// Complete profile/corrected jets, positions and directions only; no caller-supplied bounds or wire admission.
pub struct ProfileChordWorkspace<'x,'r,'a,'b,'c,'d> {
    bridge:&'x RailConstrainedBridge<'r,'a,'b,'c,'d>,
    derivatives:ProfileDerivativeCache,
}
impl ProfileChordWorkspace<'_, '_, '_, '_, '_, '_> {
    pub fn cache_hits(&self)->usize {self.derivatives.hits}
    /// Whole-rectangle exclusion using the same complete owner-bound jets
    /// as endpoint-pair queries. Cached hulls do not bypass zero-work refusal.
    pub fn inspect_self_chords(&mut self,domain:[[f64;2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.bridge.inspect_self_chords_cached(domain,sine,max_spans,&mut self.derivatives)
    }
    /// Complete owner-bound position hull shared with endpoint-pair queries.
    /// Exhaustion and invalid rectangles never borrow an earlier admission.
    pub fn enclose_position(&mut self,domain:[[f64;2];2],max_spans:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        check(max_spans<=100000,"Bound corrected position cache work")?;
        self.bridge.enclose_with_transverse_correction(domain[0],domain[1],0)?;
        if max_spans==0 {return Ok((None,0));}
        self.derivatives.corrected_position(self.bridge,domain,max_spans)
    }
    /// Complete interval derivatives on this workspace's immutable owner.
    pub fn enclose_derivatives(&mut self,domain:[[f64;2];2],max_spans:usize)
        ->Result<(Option<[[[f64;2];3];2]>,usize)> {
        check(max_spans<=100000,"Bound corrected derivative cache work")?;
        self.bridge.derivatives_with_transverse_correction(domain[0],domain[1],0)?;
        if max_spans==0 {return Ok((None,0));}
        self.derivatives.corrected_jet(self.bridge,domain,max_spans)
    }
    pub fn inspect_corrected_pair(&mut self,pair:[[[f64;2];2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.bridge.inspect_self_chord_pair_cached(pair,sine,max_spans,&mut self.derivatives)
    }
    pub fn inspect(&mut self,profile_domain:[[f64;2];2],bridge_domain:[[f64;2];2],
        high:bool,sine:f64,max_spans:usize)->Result<(Option<bool>,usize)> {
        self.bridge.inspect_profile_endpoint_chord_cached(profile_domain,bridge_domain,high,sine,max_spans,&mut self.derivatives)
    }
}
impl<'r,'a,'b,'c,'d> RailConstrainedBridge<'r,'a,'b,'c,'d> {
    pub fn profile_chord_workspace(&self)->ProfileChordWorkspace<'_,'r,'a,'b,'c,'d> {
        ProfileChordWorkspace {bridge:self,derivatives:ProfileDerivativeCache::default()}
    }
}
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
    fn self_chord_continuous(&self) -> bool {
        for high in [false,true] {
            let source=self.section(high).profile().source();
            for (knots,degree,count) in [(&source.knots_u,source.degree_u,source.control_points.len()),
                (&source.knots_v,source.degree_v,source.control_points[0].len())] {
                if degree==0 || knots.iter().any(|&k|k>knots[degree] && k<knots[count]
                    && knots.iter().filter(|&&x|x==k).count()>=degree) {return false;}
            }
        }
        true
    }
    /// Chord from this bridge's own profile to its corrected support, through
    /// an exact owned section. No foreign profile or serialized proof is accepted.
    pub fn inspect_profile_endpoint_chord(
        &self,profile_domain:[[f64;2];2],bridge_domain:[[f64;2];2],
        high:bool,sine:f64,max_spans:usize,
    )->Result<(Option<bool>,usize)> {
        self.inspect_profile_endpoint_chord_cached(profile_domain,bridge_domain,high,sine,max_spans,&mut ProfileDerivativeCache::default())
    }
    fn inspect_profile_endpoint_chord_cached(
        &self,profile_domain:[[f64;2];2],bridge_domain:[[f64;2];2],
        high:bool,sine:f64,max_spans:usize,cache:&mut ProfileDerivativeCache,
    )->Result<(Option<bool>,usize)> {
        check(max_spans<=100000 && sine.is_finite() && (0. ..1.).contains(&sine)
            && [profile_domain,bridge_domain].iter().flatten().all(|d|
                d.iter().all(|x|x.is_finite()) && d[0]<d[1]),"Bound owned cross-chart chord query")?;
        let section=self.section(high);let profile=section.profile();let axis=section.axis();let free=1-axis;
        crate::normal_alignment::validate_rectangle(profile.source(),profile_domain,max_spans.max(1))?;
        self.inspect_self_chord_pair([bridge_domain;2],sine,0)?;
        let cut=section.parameter();let end=if high {1.}else{0.};
        let mut first=profile_domain;
        first[axis]=[profile_domain[axis][0].min(cut),profile_domain[axis][1].max(cut)];
        let mut second=first;second[axis]=[cut;2];
        second[free]=[profile_domain[free][0].min(bridge_domain[0][0]),profile_domain[free][1].max(bridge_domain[0][1])];
        let union=std::array::from_fn(|k|[first[k][0].min(second[k][0]),first[k][1].max(second[k][1])]);
        profile.inspect_self_chords(union,sine,0)?;
        if max_spans==0 || !self.self_chord_continuous() {return Ok((None,0));}
        let mut used=0;let mut columns=[[[0.;2];3];3];
        for (index,domain) in [first,second].into_iter().enumerate() {
            if used==max_spans {return Ok((None,used));}
            let (jet,spent)=cache.enclose(profile,domain,max_spans-used)?;used+=spent;
            let Some(jet)=jet else {return Ok((None,used));};
            for k in 0..3 {columns[index][k]=jet[k][if index==0 {axis}else{free}];}
        }
        if used==max_spans {return Ok((None,used));}
        let across=[bridge_domain[1][0].min(end),bridge_domain[1][1].max(end)];
        let (jet,spent)=cache.corrected_jet(self,[bridge_domain[0],across],max_spans-used)?;used+=spent;
        let Some(jet)=jet else {return Ok((None,used));};columns[2]=jet[1];
        let delta=[I::point(cut).sub(I::new(profile_domain[axis][0],profile_domain[axis][1])?)?,
            I::new(bridge_domain[0][0],bridge_domain[0][1])?.sub(I::new(profile_domain[free][0],profile_domain[free][1])?)?,
            I::new(bridge_domain[1][0],bridge_domain[1][1])?.sub(I::point(end))?];
        // Exact endpoint ordering establishes the signs before interval
        // subtraction rounds an exact zero slightly across the origin.
        let positive=profile_domain[axis][1]<=cut && bridge_domain[1][0]>=end;
        let negative=profile_domain[axis][0]>=cut && bridge_domain[1][1]<=end;
        let mut transverse=[[delta[0].lo,delta[0].hi],[delta[2].lo,delta[2].hi]];
        if positive {for d in &mut transverse {d[0]=d[0].max(0.);}}
        if negative {for d in &mut transverse {d[1]=d[1].min(0.);}}
        let source_scale=(self.section(true).parameter()-self.section(false).parameter()).abs();
        let mut chord=[[0.;2];3];
        for k in 0..3 {
            let mut value=I::point(0.);
            for i in 0..3 {value=value.add(I::new(columns[i][k][0],columns[i][k][1])?.mul(delta[i])?)?;}
            chord[k]=[value.lo,value.hi];
        }
        let mut normals=[[[0.;2];3];2];
        for corrected in [false,true] {
            if used==max_spans {return Ok((None,used));}
            let (normal,spent)=if corrected {
                cache.corrected_normal(self,bridge_domain,max_spans-used)?
            }else{cache.profile_direction(profile,profile_domain,max_spans-used)?};used+=spent;
            let Some(normal)=normal else {return Ok((None,used));};
            normals[usize::from(corrected)]=normal;

            if crate::normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
                return Ok((Some(true),used));
            }
            if (positive || negative) && source_scale.is_finite() && source_scale>0.
                && crate::surface_self_chord::inspect_same_sign_transverse_chords(
                    columns,transverse,source_scale,normal,sine)? {
                return Ok((Some(true),used));
            }
        }
        // The other exact source path changes free first, then transverse.
        // Both paths reach the same section point; intersect only complete
        // independent enclosures, sharing the already paid corrected leg.
        let mut alternate=columns;let mut alternate_complete=true;
        let mut free_first=second;free_first[axis]=profile_domain[axis];
        let mut transverse_last=first;transverse_last[free]=bridge_domain[0];
        for (column,domain) in [(1,free_first),(0,transverse_last)] {
            if used==max_spans {alternate_complete=false;break;}
            let (jet,spent)=cache.enclose(profile,domain,max_spans-used)?;used+=spent;
            let Some(jet)=jet else {alternate_complete=false;break;};
            for k in 0..3 {alternate[column][k]=jet[k][if column==0 {axis}else{free}];}
        }
        if alternate_complete {
            for k in 0..3 {
                let mut value=I::point(0.);
                for i in 0..3 {value=value.add(I::new(alternate[i][k][0],alternate[i][k][1])?.mul(delta[i])?)?;}
                let overlap=I::new(chord[k][0].max(value.lo),chord[k][1].min(value.hi))?;
                chord[k]=[overlap.lo,overlap.hi];
            }
            // The alternate complete source path reaches the same owned
            // section point with the same signed parameter increments.
            // Its narrower column hulls may prove the transverse cone even
            // when the first path's hulls cannot. Never use partial paths.
            if (positive || negative) && source_scale.is_finite() && source_scale>0. {
                for normal in normals {
                    if crate::surface_self_chord::inspect_same_sign_transverse_chords(
                        alternate,transverse,source_scale,normal,sine)? {
                        return Ok((Some(true),used));
                    }
                }
            }
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
                return Ok((Some(true),used));
            }
        }
        // Endpoint subtraction independently encloses the same exact chord.
        // Refine only unresolved paths, and never use an incomplete position hull.
        if used==max_spans {return Ok((None,used));}
        let (a,spent)=cache.position(profile,profile_domain,max_spans-used)?;used+=spent;
        let Some(a)=a else {return Ok((None,used));};
        if used==max_spans {return Ok((None,used));}
        let (b,spent)=cache.corrected_position(self,bridge_domain,max_spans-used)?;used+=spent;
        let Some(b)=b else {return Ok((None,used));};
        for k in 0..3 {
            let difference=I::new(b[k][0],b[k][1])?.sub(I::new(a[k][0],a[k][1])?)?;
            let overlap=I::new(chord[k][0].max(difference.lo),chord[k][1].min(difference.hi))?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
                return Ok((Some(true),used));
            }
        }
        Ok((Some(false),used))
    }
    /// Fresh whole-path derivatives and local endpoint normals of the exact
    /// corrected expression, with continuous source derivative fields.
    pub fn inspect_self_chord_pair(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,
    ) -> Result<(Option<bool>,usize)> {
        self.inspect_self_chord_pair_cached(pair,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
    fn inspect_self_chord_pair_cached(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>,usize)> {
        check(max_spans<=100000 && pair.iter().all(|uv|uv.iter().all(|d|
            d.iter().all(|x|x.is_finite()) && d[0]<d[1])),"Bound corrected chord endpoints and work")?;
        let union=std::array::from_fn(|k|[pair[0][k][0].min(pair[1][k][0]),pair[0][k][1].max(pair[1][k][1])]);
        self.inspect_self_chords(union,max_sine_squared,0)?;
        if max_spans==0 || !self.self_chord_continuous() {return Ok((None,0));}
        let (jet,mut spent)=cache.corrected_jet(self,union,max_spans)?;
        let Some(jet)=jet else {return Ok((None,spent));};
        let mut jacobian=std::array::from_fn(|k|std::array::from_fn(|axis|jet[axis][k]));
        if !crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)? {
            // Same two-leg path as exact profiles; every column comes from
            // the corrected expression on its entire connecting strip.
            let strips=[[union[0],pair[0][1]],[pair[1][0],union[1]]];
            let mut columns=jacobian;
            for axis in 0..2 {
                if spent==max_spans {return Ok((None,spent));}
                let (bounds,used)=cache.corrected_jet(self,strips[axis],max_spans-spent)?;
                spent+=used;
                let Some(bounds)=bounds else {return Ok((None,spent));};
                for k in 0..3 {columns[k][axis]=bounds[axis][k];}
            }
            jacobian=columns;
        }
        // Direct M*(B-A) interval exclusion proves its own nonzero chord;
        // only the mean-matrix theorem requires uniform rank admission.
        let ranked=crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)?;
        let mut normals=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            let (normal,used)=cache.corrected_normal(self,endpoint,max_spans-spent)?;
            spent+=used;
            let Some(normal)=normal else {return Ok((None,spent));};
            normals[index]=normal;
            if crate::surface_self_chord::inspect_jacobian_endpoint_pair_chords(jacobian,pair,normal,max_sine_squared)?
                || (ranked && crate::surface_self_chord::inspect_jacobian_endpoint_chords(jacobian,normal,max_sine_squared)?.0) {
                return Ok((Some(true),spent));
            }
        }
        // Even a full-rank union can be wider than its coordinate strips.
        // Unresolved pairs therefore use the first path regardless of rank.
        let strips=[[union[0],pair[0][1]],[pair[1][0],union[1]]];
        for axis in 0..2 {
            if spent==max_spans {return Ok((None,spent));}
            let (jet,used)=cache.corrected_jet(self,strips[axis],max_spans-spent)?;spent+=used;
            let Some(jet)=jet else {return Ok((None,spent));};
            for k in 0..3 {jacobian[k][axis]=jet[axis][k];}
        }
        // A complete first path already encloses the exact endpoint chord.
        // Admit its exclusion before paying for the alternate path; uncertainty
        // still falls through to the intersection of both complete bounds.
        for normal in normals {
            if crate::surface_self_chord::inspect_jacobian_endpoint_pair_chords(
                jacobian,pair,normal,max_sine_squared)? {
                return Ok((Some(true),spent));
            }
        }
        // Reverse the coordinate path: across first at A.free, then free
        // at B.across. Both complete paths enclose the same exact chord.
        let strips=[[union[0],pair[1][1]],[pair[0][0],union[1]]];
        let mut alternate=jacobian;
        for axis in 0..2 {
            if spent==max_spans {return Ok((None,spent));}
            let (jet,used)=cache.corrected_jet(self,strips[axis],max_spans-spent)?;spent+=used;
            let Some(jet)=jet else {return Ok((None,spent));};
            for k in 0..3 {alternate[k][axis]=jet[axis][k];}
        }
        let mut delta=[I::point(0.);2];
        for axis in 0..2 {
            delta[axis]=I::new(pair[1][axis][0],pair[1][axis][1])?
                .sub(I::new(pair[0][axis][0],pair[0][axis][1])?)?;
        }
        let mut chord=[[0.;2];3];
        for k in 0..3 {
            let evaluate=|j:[[f64;2];2]| -> Result<I> {
                I::new(j[0][0],j[0][1])?.mul(delta[0])?
                    .add(I::new(j[1][0],j[1][1])?.mul(delta[1])?)
            };
            let first=evaluate(jacobian[k])?;let second=evaluate(alternate[k])?;
            let overlap=first.intersect(second.lo,second.hi)?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                return Ok((Some(true),spent));
            }
        }
        // Independently enclose both positions on the same corrected owner.
        // Only complete endpoint hulls may tighten the already bounded chord.
        let mut positions=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            let (position,used)=cache.corrected_position(self,endpoint,max_spans-spent)?;spent+=used;
            let Some(position)=position else {return Ok((None,spent));};
            positions[index]=position;
        }
        for k in 0..3 {
            let difference=I::new(positions[1][k][0],positions[1][k][1])?
                .sub(I::new(positions[0][k][0],positions[0][k][1])?)?;
            let overlap=I::new(chord[k][0],chord[k][1])?.intersect(difference.lo,difference.hi)?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                return Ok((Some(true),spent));
            }
        }
        Ok((Some(false),spent))
    }
    /// Diagnostic exclusion on the exact transverse-corrected expression.
    /// Its source derivative fields must be continuous before a mean bound
    /// can connect arbitrary endpoints on the rectangle.
    pub fn inspect_self_chords(
        &self, domain: [[f64; 2]; 2], max_sine_squared: f64, max_spans: usize,
    ) -> Result<(Option<bool>, usize)> {
        self.inspect_self_chords_cached(domain,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
    fn inspect_self_chords_cached(
        &self, domain: [[f64; 2]; 2], max_sine_squared: f64, max_spans: usize,
        cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>, usize)> {
        let range = self.section(false).range();
        check(max_spans <= 100000 && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && domain.iter().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<d[1])
            && domain[0][0]>=range[0] && domain[0][1]<=range[1]
            && domain[1][0]>=0. && domain[1][1]<=1.,
            "Bound corrected self-chord domain, angle and work")?;
        if !self.self_chord_continuous() {return Ok((None,0));}
        if max_spans==0 {return Ok((None,0));}
        let (jet,spent)=cache.corrected_jet(self,domain,max_spans)?;
        let Some(jet)=jet else {return Ok((None,spent));};
        let jacobian=std::array::from_fn(|k|std::array::from_fn(|axis|jet[axis][k]));
        Ok((Some(crate::surface_self_chord::inspect_jacobian_self_chords(jacobian,max_sine_squared)?.0),spent))
    }
    fn mean_radius_residual(
        &self,
        centers: &Curve,
        radius: &Curve,
        domains: [[f64; 2]; 2],
        rect: [[f64; 2]; 2],
        position: [[f64; 2]; 3],
        c: &[I],
        r: I,
        budget: usize,
    ) -> Result<(Option<([f64; 2], [f64; 2])>, usize)> {
        for (curve, domain) in [centers, radius].into_iter().zip(domains) {
            if !continuous_knots(
                &curve.knots,
                curve.degree,
                curve.control_points.len(),
                domain,
            ) {
                return Ok((None, 0));
            }
        }
        // The exact profile rails participate in the corrected expression.
        // A discontinuous source chart cannot use a whole-cell mean bound.
        for high in [false, true] {
            let section = self.section(high);
            let source = section.profile().source();
            let across = [
                self.section(false).parameter(),
                self.section(true).parameter(),
            ];
            let mut domain = [[across[0].min(across[1]), across[0].max(across[1])]; 2];
            domain[1 - section.axis()] = rect[0];
            for (axis, (knots, degree, count)) in [
                (
                    &source.knots_u,
                    source.degree_u,
                    source.control_points.len(),
                ),
                (
                    &source.knots_v,
                    source.degree_v,
                    source.control_points[0].len(),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                if !continuous_knots(knots, degree, count, domain[axis]) {
                    return Ok((None, 0));
                }
            }
        }
        let midpoint = rect.map(|d| d[0] * 0.5 + d[1] * 0.5);
        let (p, mut spent) = self.enclose_with_transverse_correction(
            [midpoint[0]; 2],
            [midpoint[1]; 2],
            budget.min(100000),
        )?;
        let Some(p) = p else {
            return Ok((None, spent));
        };
        let ct = mapped(domains[0], [midpoint[1]; 2])?;
        let rt = mapped(domains[1], [midpoint[1]; 2])?;
        let used = spans(centers, ct) + spans(radius, rt);
        if used > budget - spent {
            return Ok((None, spent));
        }
        spent += used;
        let center = interval_eval::evaluate_interval(centers, ct)?;
        let law = interval_eval::evaluate_interval(radius, rt)?;
        let point = crate::moving_radius::bound_expression(
            p,
            std::array::from_fn(|k| [center[k].lo, center[k].hi]),
            [law[0].lo.max(0.), law[0].hi],
        )?;
        let (jet, used) = self.derivatives_with_transverse_correction(
            rect[0],
            rect[1],
            (budget - spent).min(100000),
        )?;
        spent += used;
        let Some(jet) = jet else {
            return Ok((None, spent));
        };
        let (cd, used) = law_derivative(
            centers,
            mapped(domains[0], rect[1])?,
            domains[0],
            budget - spent,
        )?;
        spent += used;
        let Some(cd) = cd else {
            return Ok((None, spent));
        };
        let (rd, used) = law_derivative(
            radius,
            mapped(domains[1], rect[1])?,
            domains[1],
            budget - spent,
        )?;
        spent += used;
        let Some(rd) = rd else {
            return Ok((None, spent));
        };
        let mut gradient = [I::point(0.); 2];
        for k in 0..3 {
            let delta = I::new(position[k][0], position[k][1])?.sub(c[k])?;
            gradient[0] = gradient[0].add(
                I::point(2.)
                    .mul(delta)?
                    .mul(I::new(jet[0][k][0], jet[0][k][1])?)?,
            )?;
            gradient[1] = gradient[1].add(
                I::point(2.)
                    .mul(delta)?
                    .mul(I::new(jet[1][k][0], jet[1][k][1])?.sub(cd[k])?)?,
            )?;
        }
        gradient[1] = gradient[1].sub(I::point(2.).mul(r)?.mul(rd[0])?)?;
        let mut residual = I::new(point.squared_residual[0], point.squared_residual[1])?;
        let mut contributions = [0.; 2];
        for axis in 0..2 {
            let contribution = gradient[axis]
                .mul(I::new(rect[axis][0], rect[axis][1])?.sub(I::point(midpoint[axis]))?)?;
            contributions[axis] = contribution.lo.abs().max(contribution.hi.abs());
            residual = residual.add(contribution)?;
        }
        Ok((Some(([residual.lo, residual.hi], contributions)), spent))
    }
    /// Center and radius curves are mapped over their complete domains by
    /// normalized across coordinate. Radius uses coordinate zero. All free
    /// coordinates participate; no sample or unchanged source admits a cell.
    pub fn inspect_radius(
        &self,
        centers: &Curve,
        radius: &Curve,
        tolerance: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<Report> {
        centers.validate()?;
        radius.validate()?;
        self.inspect_radius_on_domains(
            centers,
            radius,
            [centers.domain(), radius.domain()],
            tolerance,
            max_cells,
            max_spans,
        )
    }
    /// Restrict the original law parameters without rebuilding either curve.
    /// Each ordered domain maps affinely from the entire across interval.
    pub fn inspect_radius_on_domains(
        &self,
        centers: &Curve,
        radius: &Curve,
        domains: [[f64; 2]; 2],
        tolerance: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<Report> {
        centers.validate()?;
        radius.validate()?;
        for (curve, domain) in [centers, radius].into_iter().zip(domains) {
            let full = curve.domain();
            check(
                domain.iter().all(|x| x.is_finite())
                    && full[0] <= domain[0]
                    && domain[0] < domain[1]
                    && domain[1] <= full[1],
                "Keep original radius-law domain ordered inside its curve",
            )?;
        }
        check(
            centers.control_points[0].len() == 3
                && radius.control_points[0].len() == 2
                && radius.control_points.iter().all(|p| p[0] >= 0.)
                && tolerance.is_finite()
                && tolerance > 0.
                && max_cells <= 100000
                && max_spans <= 1000000,
            "Bound exact expression radius law and work",
        )?;
        let range = self.section(false).range();
        let mut out = Report {
            domain: [range, [0., 1.]],
            complete: false,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            accepted_error_upper: 0.,
            pending: vec![[range, [0., 1.]]],
            reason: "expression-radius-work-limit",
            violation: None,
        };
        while let Some(rect) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                return Ok(out);
            }
            out.cells += 1;
            let (position, used) = self.enclose_with_transverse_correction(
                rect[0],
                rect[1],
                (max_spans - out.spans).min(100000),
            )?;
            out.spans += used;
            let Some(position) = position else {
                out.pending.push(rect);
                return Ok(out);
            };
            let ct = mapped(domains[0], rect[1])?;
            let rt = mapped(domains[1], rect[1])?;
            let used = spans(centers, ct) + spans(radius, rt);
            if used > max_spans - out.spans {
                out.pending.push(rect);
                return Ok(out);
            }
            out.spans += used;
            let c = interval_eval::evaluate_interval(centers, ct)?;
            let r = interval_eval::evaluate_interval(radius, rt)?;
            let mut bound = crate::moving_radius::bound_expression(
                position,
                std::array::from_fn(|axis| [c[axis].lo, c[axis].hi]),
                [r[0].lo.max(0.), r[0].hi],
            )?;
            let mut split_contributions = None;
            if bound.error_upper > tolerance && max_spans - out.spans >= 100 {
                let (residual, spent) = self.mean_radius_residual(
                    centers,
                    radius,
                    domains,
                    rect,
                    position,
                    &c,
                    r[0],
                    max_spans - out.spans,
                )?;
                out.spans += spent;
                if let Some((residual, contributions)) = residual {
                    split_contributions = Some(contributions);
                    bound = crate::moving_radius::refine_expression_bound(
                        bound,
                        residual,
                        [r[0].lo.max(0.), r[0].hi],
                    )?;
                }
            }
            // A rigorously enclosed point may refute the law, never admit the
            // rectangle. Probe initial interior points before depth-first coverage
            // spends all work near a boundary with zero residual.
            if out.cells == 1 && bound.error_upper > tolerance && max_spans - out.spans >= 34 {
                let interior = rect.map(|d| {
                    let middle = d[0] * 0.5 + d[1] * 0.5;
                    [middle, d[0] * 0.5 + middle * 0.5, middle * 0.5 + d[1] * 0.5]
                });
                for probe_index in 0..9 {
                    if max_spans - out.spans < 34 {
                        break;
                    }
                    let point = [
                        [interior[0][probe_index / 3]; 2],
                        [interior[1][probe_index % 3]; 2],
                    ];
                    let (p, used) = self.enclose_with_transverse_correction(
                        point[0],
                        point[1],
                        (max_spans - out.spans).min(100000),
                    )?;
                    out.spans += used;
                    if let Some(p) = p {
                        let ct = mapped(domains[0], point[1])?;
                        let rt = mapped(domains[1], point[1])?;
                        let used = spans(centers, ct) + spans(radius, rt);
                        if used <= max_spans - out.spans {
                            out.spans += used;
                            let c = interval_eval::evaluate_interval(centers, ct)?;
                            let r = interval_eval::evaluate_interval(radius, rt)?;
                            let probe = crate::moving_radius::bound_expression(
                                p,
                                std::array::from_fn(|axis| [c[axis].lo, c[axis].hi]),
                                [r[0].lo.max(0.), r[0].hi],
                            )?;
                            if probe.error_lower > tolerance {
                                out.violation = Some(Violation {
                                    domain: point,
                                    error_bounds: [probe.error_lower, probe.error_upper],
                                    squared_residual: probe.squared_residual,
                                });
                                out.pending.push(rect);
                                out.reason = "expression-radius-law-mismatch";
                                return Ok(out);
                            }
                        }
                    }
                }
            }
            if bound.error_upper <= tolerance {
                out.accepted_cells += 1;
                out.accepted_error_upper = out.accepted_error_upper.max(bound.error_upper);
                continue;
            }
            if bound.error_lower > tolerance {
                out.violation = Some(Violation {
                    domain: rect,
                    error_bounds: [bound.error_lower, bound.error_upper],
                    squared_residual: bound.squared_residual,
                });
                out.pending.push(rect);
                out.reason = "expression-radius-law-mismatch";
                return Ok(out);
            }
            let balanced_axis = usize::from(
                rect[1][1] - rect[1][0] > (rect[0][1] - rect[0][0]) / (range[1] - range[0]),
            );
            // Split the dominant whole-cell residual contribution. This only
            // chooses coverage order; every child still requires interval proof.
            let axis = split_contributions
                .filter(|c| c[0] != c[1])
                .map_or(balanced_axis, |c| usize::from(c[1] > c[0]));
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                out.reason = "expression-radius-resolution-limit";
                return Ok(out);
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            out.pending.push(right);
            out.pending.push(left);
        }
        out.complete = true;
        out.reason = "expression-radius-complete-coverage";
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cross_profile_path_preserves_exact_normal_diameter() {
        // Positive-weight single-span rational cubic with opposite endpoints
        // and vertical endpoint tangents: the diameter is normal at both ends.
        let surface=crate::surface::Surface {
            degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],
            knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:[0.,1.].into_iter().map(|x|
                [[1.,0.],[1.,2.],[-1.,2.],[-1.,0.]].into_iter().map(|p|vec![x,p[0],p[1]]).collect()).collect(),
            weights:vec![vec![1.,1./3.,1./3.,1.];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
        let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let a=[[0.4,0.41],[0.,0.001]];let b=[[0.999,1.],[0.4,0.41]];
        let diameter=[[[0.,0.001],[0.4,0.41]],b];
        let mut self_workspace=corrected.profile_chord_workspace();
        assert!(self_workspace.inspect_corrected_pair(diameter,1e-4,1000).unwrap().0!=Some(true));
        for budget in [0,1,64,1000] {
            let (result,spent)=self_workspace.inspect_corrected_pair(diameter,1e-4,budget).unwrap();
            assert!(result!=Some(true) && spent<=budget);
            let reverse=[diameter[1],diameter[0]];
            let (result,spent)=self_workspace.inspect_corrected_pair(reverse,1e-4,budget).unwrap();
            assert!(result!=Some(true) && spent<=budget,
                "reversing the coordinate paths must preserve an exact normal diameter");
        }
        let mut workspace=corrected.profile_chord_workspace();
        for high in [false,true,false,true] {
            assert!(workspace.inspect(a,b,high,1e-4,1000).unwrap().0!=Some(true));
            for budget in [0,1,64,1000] {
                let (cached,cost)=workspace.inspect(a,b,high,1e-4,budget).unwrap();
                assert!(cached!=Some(true) && cost<=budget);
                let (excluded,spent)=corrected.inspect_profile_endpoint_chord(a,b,high,1e-4,budget).unwrap();
                assert!(excluded!=Some(true));assert!(spent<=budget);
            }
        }
    }
    #[test]
    fn cross_profile_cache_allows_larger_budget_direction_retry() {
        let surface=crate::surface::Surface {
            degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],
                vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.,4.];3],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
        let domain=[[0.01,0.99],[0.5,0.5]];let mut cache=ProfileDerivativeCache::default();
        let small=cache.profile_direction(&profile,domain,1).unwrap();
        assert!(small.0.unwrap().iter().all(|d|d[0]<=0. && d[1]>=0.));
        assert!(cache.directions.is_empty());
        let large=cache.profile_direction(&profile,domain,100).unwrap();
        assert!(large.0.unwrap()[2][0]>0. && large.1>small.1);
        assert_eq!(cache.profile_direction(&profile,domain,1).unwrap(),(large.0,0));
    }
    #[test]
    fn cross_profile_cache_does_not_mix_equal_uv_on_distinct_owners() {
        let first=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,1.,0.]]).collect(),
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
        };
        let mut second=first.clone();
        for row in &mut second.control_points {for p in row {p[2]=10.+2.*p[0];}}
        let a=crate::surface_weight_profile::qualify_weight_profile(&first,[0,0],100).unwrap().0.unwrap();
        let b=crate::surface_weight_profile::qualify_weight_profile(&second,[0,0],100).unwrap().0.unwrap();
        let independent=crate::normal_alignment::ProfileSectionBridge::new(
            a.section(0,0.).unwrap(),b.section(0,1.).unwrap(),[1.;2]).unwrap();
        assert!(independent.constrain_original_rails(100).unwrap().0.is_none(),
            "The current public corrected bridge refuses different profile owners");
        let twin=crate::surface_weight_profile::qualify_weight_profile(&first,[0,0],100).unwrap().0.unwrap();
        let equal_geometry=crate::normal_alignment::ProfileSectionBridge::new(
            a.section(0,0.).unwrap(),twin.section(0,1.).unwrap(),[1.;2]).unwrap();
        assert!(equal_geometry.constrain_original_rails(100).unwrap().0.is_none(),
            "Equal source geometry still does not transfer profile ownership");
        let domain=[[0.1,0.9];2];let mut cache=ProfileDerivativeCache::default();
        let ja=a.derivative_bounds(domain,100).unwrap().0;
        let jb=b.derivative_bounds(domain,100).unwrap().0;
        assert_ne!(ja,jb);
        assert_eq!(cache.enclose(&a,domain,100).unwrap().0,ja);
        assert_eq!(cache.enclose(&b,domain,100).unwrap().0,jb);
        for owner in [&a,&b,&a,&b] {
            assert_eq!(cache.enclose(owner,domain,100).unwrap().0,owner.derivative_bounds(domain,100).unwrap().0);
            assert_eq!(cache.position(owner,domain,100).unwrap().0,owner.enclose(domain,100).unwrap().0);
            assert_eq!(cache.profile_direction(owner,domain,100).unwrap().0,owner.interior_normal_direction_bounds(domain,100).unwrap().0);
        }
        assert!(cache.hits>0);
    }
    #[test]
    fn cross_profile_cache_never_stores_incomplete_span_hulls() {
        let surface=crate::surface::Surface {
            degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,0.5,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:[0.,0.25,0.75,1.].into_iter().map(|x|
                vec![vec![x,0.,0.],vec![x,1.,0.]]).collect(),
            weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
        let mut cache=ProfileDerivativeCache::default();let domain=[[0.,1.];2];
        assert_eq!(cache.enclose(&profile,domain,1).unwrap(),(None,1));
        assert!(cache.entries.is_empty());
        let complete=cache.enclose(&profile,domain,2).unwrap();assert!(complete.0.is_some());
        assert_eq!(complete.1,2);assert_eq!(cache.hits,0);
        assert_eq!(cache.enclose(&profile,domain,1).unwrap(),(complete.0,0));
        assert_eq!(cache.hits,1);
        assert_eq!(cache.position(&profile,domain,1).unwrap(),(None,1));
        assert!(cache.positions.is_empty());
        let position=cache.position(&profile,domain,2).unwrap();assert!(position.0.is_some());
        assert_eq!(position.1,2);
        assert_eq!(cache.position(&profile,domain,1).unwrap(),(position.0,0));
    }
    #[test]
    fn cross_profile_path_refuses_unqualified_free_derivative_seam() {
        let surface=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],
            knots_v:vec![0.,0.,0.5,1.,1.],
            control_points:[0.,1.].into_iter().map(|x|
                [0.,0.4,1.].into_iter().map(|y|vec![x,y,0.]).collect()).collect(),
            weights:vec![vec![1.;3];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
        assert!(bridge.constrain_original_rails(100).unwrap().0.is_none(),
            "unadmitted source derivative seams cannot supply a corrected cross-chart owner");
        let mut disconnected=surface.clone();
        disconnected.knots_v=vec![0.,0.,0.5,0.5,1.,1.];
        for row in &mut disconnected.control_points {row.insert(2,vec![row[0][0],0.6,0.]);}
        disconnected.weights=vec![vec![1.;4];2];
        assert!(disconnected.validate().is_err());
    }
    #[test]
    fn cross_profile_path_preserves_nonunit_parameter_domains() {
        let surface=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![2.,2.,4.,4.],knots_v:vec![-3.,-3.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
                vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],4).unwrap().0.unwrap();
        for axis in [0,1] {
            let cuts=if axis==0 {[3.,3.5]}else{[-2.,-1.]};
            let bridge=crate::normal_alignment::ProfileSectionBridge::new(
                profile.section(axis,cuts[0]).unwrap(),profile.section(axis,cuts[1]).unwrap(),[cuts[1]-cuts[0];2]).unwrap();
            let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
            let source=[[2.125,2.25],[-2.75,-2.5]];
            let target=[if axis==0 {[-0.25,0.]}else{[3.5,3.75]},[0.25,0.5]];
            for high in [false,true] {
                let (excluded,spent)=corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,1000).unwrap();
                assert_eq!(excluded,Some(true));assert!(spent<=1000);
                assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
            }
        }
    }
    #[test]
    fn owned_cross_chart_proof_covers_zero_separation_at_both_seam_ends() {
        let surface=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],4).unwrap().0.unwrap();
        for axis in [0,1] {
            let bridge=crate::normal_alignment::ProfileSectionBridge::new(
                profile.section(axis,0.4).unwrap(),profile.section(axis,0.6).unwrap(),[0.6-0.4;2]).unwrap();
            let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
            for high in [false,true] {for epsilon in [0.1,1e-6] {
                let mut source=[[0.45,0.55];2];
                source[axis]=if high {[0.6,0.6+epsilon]}else{[0.4-epsilon,0.4]};
                let target=[[0.45,0.55],if high {[1.-epsilon,1.]}else{[0.,epsilon]}];
                let a=profile.enclose(source,1000).unwrap().0.unwrap();
                let b=corrected.enclose_with_transverse_correction(target[0],target[1],1000).unwrap().0.unwrap();
                let chord=std::array::from_fn(|k| {
                    let d=I::new(b[k][0],b[k][1]).unwrap().sub(I::new(a[k][0],a[k][1]).unwrap()).unwrap();
                    [d.lo,d.hi]
                });
                assert!(chord.iter().all(|d|d[0]<=0. && d[1]>=0.));
                assert_ne!(crate::normal_alignment::inspect_pair_bounds([
                    [[0.,0.],[0.,0.],[1.,1.]],chord],1e-4).unwrap().aligned,Some(false));
                assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,1000).unwrap().0,Some(true));
                assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
            }}
        }
    }
    #[test]
    fn owned_cross_chart_proof_covers_curved_seam_zero_separation() {
        for rational in [false,true] {
        let surface=crate::surface::Surface {
            degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
                vec![vec![0.5,0.,0.],vec![0.5,1.,0.]],
                vec![vec![1.,0.,0.01],vec![1.,1.,0.01]]],
            weights:if rational {vec![vec![1.,1.125],vec![1.25,1.40625],vec![1.,1.125]]} else {vec![vec![1.;2];3]},periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
        for axis in [0,1] {
            let bridge=crate::normal_alignment::ProfileSectionBridge::new(
                profile.section(axis,0.4).unwrap(),profile.section(axis,0.6).unwrap(),[0.6-0.4;2]).unwrap();
            let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
            for high in [false,true] {for epsilon in [0.1,1e-6] {
                let mut source=[[0.45,0.55];2];
                source[axis]=if high {[0.6,0.6+epsilon]}else{[0.4-epsilon,0.4]};
                let target=[[0.45,0.55],if high {[1.-epsilon,1.]}else{[0.,epsilon]}];
                let a=profile.enclose(source,1000).unwrap().0.unwrap();
                let b=corrected.enclose_with_transverse_correction(target[0],target[1],1000).unwrap().0.unwrap();
                let chord=std::array::from_fn(|k| {
                    let d=I::new(b[k][0],b[k][1]).unwrap().sub(I::new(a[k][0],a[k][1]).unwrap()).unwrap();
                    [d.lo,d.hi]
                });
                assert!(chord.iter().all(|d|d[0]<=0. && d[1]>=0.));
                let normal=profile.interior_normal_direction_bounds(source,1000).unwrap().0.unwrap();
                assert_ne!(crate::normal_alignment::inspect_pair_bounds([normal,chord],1e-4).unwrap().aligned,Some(false));
                assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,1000).unwrap().0,Some(true));
                assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
            }}
        }
    }
    }
    #[test]
    fn owned_profile_transition_chord_uses_exact_section_and_budget() {
        let surface=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],
                vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],4).unwrap().0.unwrap();
        for axis in [0,1] {
            let bridge=crate::normal_alignment::ProfileSectionBridge::new(
                profile.section(axis,0.4).unwrap(),profile.section(axis,0.6).unwrap(),[0.6-0.4;2]).unwrap();
            let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
            let mut shared=corrected.profile_chord_workspace();
            let shared_domain=[[0.1,0.2],[0.3,0.4]];
            assert_eq!(shared.enclose_position(shared_domain,1).unwrap(),(None,1));
            let position=shared.enclose_position(shared_domain,1000).unwrap();
            assert_eq!(position,corrected.enclose_with_transverse_correction(shared_domain[0],shared_domain[1],1000).unwrap());
            assert!(position.0.is_some());
            assert_eq!(shared.enclose_position(shared_domain,1).unwrap(),(position.0,0));
            assert_eq!(shared.enclose_position(shared_domain,0).unwrap(),(None,0));
            let partial_jet=shared.enclose_derivatives(shared_domain,1).unwrap();
            assert!(partial_jet.0.is_none() && partial_jet.1<=1);
            assert!(shared.derivatives.corrected.is_empty());
            let derivatives=shared.enclose_derivatives(shared_domain,1000).unwrap();
            assert_eq!(derivatives,corrected.derivatives_with_transverse_correction(shared_domain[0],shared_domain[1],1000).unwrap());
            assert!(derivatives.0.is_some());
            assert_eq!(shared.enclose_derivatives(shared_domain,1).unwrap(),(derivatives.0,0));
            assert_eq!(shared.enclose_derivatives(shared_domain,0).unwrap(),(None,0));
            for invalid in [[[-0.1,0.2],[0.3,0.4]],[[f64::NAN,0.2],[0.3,0.4]]] {
                assert!(shared.enclose_position(invalid,0).is_err());
                assert!(shared.enclose_derivatives(invalid,0).is_err());
            }
            assert!(shared.enclose_position(shared_domain,100001).is_err());
            assert!(shared.enclose_derivatives(shared_domain,100001).is_err());
            let mut position_cache=ProfileDerivativeCache::default();
            let position_domain=[[0.1,0.2],[0.3,0.4]];
            let incomplete=position_cache.corrected_position(&corrected,position_domain,1).unwrap();
            assert_eq!(incomplete,(None,1));assert!(position_cache.corrected_positions.is_empty());
            let complete=position_cache.corrected_position(&corrected,position_domain,1000).unwrap();
            assert!(complete.0.is_some());assert!(complete.1>1 && complete.1<=1000);
            assert_eq!(position_cache.corrected_position(&corrected,position_domain,1).unwrap(),(complete.0,0));
            let pair=[[[0.1,0.2];2],[[0.8,0.9];2]];
            let mut self_workspace=corrected.profile_chord_workspace();
            let cold=self_workspace.inspect_corrected_pair(pair,1e-4,1000).unwrap();
            assert_eq!(cold.0,Some(true));
            assert_eq!(cold,corrected.inspect_self_chord_pair(pair,1e-4,1000).unwrap());
            let union=[[0.1,0.9];2];
            let direct_intrinsic=corrected.inspect_self_chords(union,1e-4,1000).unwrap();
            assert!(direct_intrinsic.0.is_some() && direct_intrinsic.1>0);
            let shared_intrinsic=self_workspace.inspect_self_chords(union,1e-4,1000).unwrap();
            assert_eq!(shared_intrinsic,(direct_intrinsic.0,0));
            assert_eq!(self_workspace.inspect_self_chords(union,1e-4,0).unwrap(),(None,0));
            assert!(self_workspace.inspect_self_chords([[-0.1,0.9];2],1e-4,0).is_err());
            assert!(self_workspace.inspect_self_chords(union,f64::NAN,0).is_err());
            let mut fresh_intrinsic=corrected.profile_chord_workspace();
            assert_eq!(fresh_intrinsic.inspect_self_chords(union,1e-4,1000).unwrap(),direct_intrinsic);

            let mut seeded=corrected.profile_chord_workspace();
            for endpoint in pair {
                assert!(seeded.enclose_position(endpoint,1000).unwrap().0.is_some());
                assert!(seeded.enclose_derivatives(endpoint,1000).unwrap().0.is_some());
            }
            let shared_pair=seeded.inspect_corrected_pair(pair,1e-4,1000).unwrap();
            assert_eq!(shared_pair.0,cold.0);
            assert!(shared_pair.1<cold.1,"Endpoint-pair work must reuse the already paid wall enclosures");
            assert!(seeded.cache_hits()>0);
            assert_eq!(self_workspace.inspect_corrected_pair(pair,1e-4,1).unwrap(),(cold.0,0));
            assert_eq!(self_workspace.inspect_corrected_pair(pair,1e-4,0).unwrap(),(None,0));
            let source=[[0.1,0.2];2];let target=[[0.7,0.8],[0.3,0.4]];
            for high in [false,true] {
                let (excluded,spent)=corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,100).unwrap();
                assert_eq!(excluded,Some(true));assert!(spent<=100);
                assert!(spent<66,"The complete first path must exclude before optional alternate-path work");
                let mut workspace=corrected.profile_chord_workspace();
                let first=workspace.inspect(source,target,high,1e-4,1000).unwrap();
                let second=workspace.inspect(source,target,high,1e-4,1000).unwrap();
                assert_eq!(first.0,second.0);assert!(second.1<first.1);
                assert!(workspace.cache_hits()>0);
                assert_eq!(workspace.inspect(source,target,high,1e-4,0).unwrap(),(None,0));
                assert!(workspace.inspect([[-0.1,0.2];2],target,high,1e-4,0).is_err());
                let mut independent=corrected.profile_chord_workspace();
                assert_eq!(independent.inspect(source,target,high,1e-4,1000).unwrap(),first);
                let mut partial=ProfileDerivativeCache::default();
                let exhausted=partial.corrected_jet(&corrected,target,1).unwrap();
                assert!(exhausted.0.is_none() && exhausted.1<=1);
                assert!(partial.corrected.is_empty());
                let completed=partial.corrected_jet(&corrected,target,1000).unwrap();
                assert!(completed.0.is_some());
                assert_eq!(partial.corrected_jet(&corrected,target,1).unwrap(),(completed.0,0));
                let actual=corrected.normals_with_transverse_correction(target[0],target[1],1000).unwrap().0;
                assert_eq!(workspace.derivatives.corrected_normal(&corrected,target,1000).unwrap().0,actual);
                assert_eq!(workspace.derivatives.corrected_normal(&corrected,target,1).unwrap(),(actual,0));
                let mut overlap=[[0.;2];2];overlap[axis]=[0.46,0.48];overlap[1-axis]=target[0];
                assert_eq!(corrected.inspect_profile_endpoint_chord(overlap,target,high,1e-4,1000).unwrap().0,Some(false));
                assert_eq!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,0).unwrap(),(None,0));
                for budget in 1..=8 {
                    let (result,used)=corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,budget).unwrap();
                    assert!(used<=budget);
                    if used<spent {assert!(result!=Some(true));}
                }
                assert!(corrected.inspect_profile_endpoint_chord(source,target,high,1.,0).is_err());
                assert!(corrected.inspect_profile_endpoint_chord(source,target,high,1e-4,100001).is_err());
                assert!(corrected.inspect_profile_endpoint_chord([[0.1;2];2],target,high,1e-4,0).is_err());
                assert!(corrected.inspect_profile_endpoint_chord([[-0.1,0.2];2],target,high,1e-4,0).is_err());
                assert!(corrected.inspect_profile_endpoint_chord(source,[[0.7,0.8],[0.9,1.1]],high,1e-4,0).is_err());
            }
        }
    }
    #[test]
    fn law_jets_preserve_nonunit_parameter_scaling_and_work_refusal() {
        let curve = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 4.],
            control_points: vec![vec![1., 2., 3.], vec![5., 6., 7.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        curve.validate().unwrap();
        let (derivative, spent) =
            law_derivative(&curve, I::new(2.5, 3.5).unwrap(), [2.5, 3.5], 1).unwrap();
        assert_eq!(spent, 1);
        for d in derivative.unwrap() {
            assert!(d.lo <= 2. && d.hi >= 2.);
            assert!(d.hi - d.lo < 1e-10);
        }
        assert!(
            law_derivative(&curve, I::new(2.5, 3.5).unwrap(), [2.5, 3.5], 0)
                .unwrap()
                .0
                .is_none()
        );
    }
    #[test]
    fn endpoint_position_refinement_preserves_translated_scale() {
        let w=std::f64::consts::FRAC_1_SQRT_2;
        for scale in [1e-3,1.,1e3] {
            for far in [false,true] {
            let shift=if far {[1e6,-2e6,3e6]}else{[0.;3]};
            let surface=crate::surface::Surface {
                degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],
                knots_v:vec![0.,0.,0.,1.,1.,1.],
                control_points:[0.,1.].into_iter().map(|x|
                    [[x,1.,0.],[x,1.,1.],[x,0.,1.]].into_iter().map(|p|
                        (0..3).map(|k|shift[k]+scale*p[k]).collect()).collect()).collect(),
                weights:vec![vec![1.,w,1.];2],periodic_u:false,periodic_v:false,
            };
            let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
            // Exact extrusion derivative dX/du=scale; translation must not
            // widen its enclosure by the absolute coordinate magnitude.
            let (jet,spent)=profile.derivative_bounds([[0.1,0.11],[0.1,0.11]],100).unwrap();
            let dx=jet.unwrap()[0][0];assert_eq!(spent,1);
            assert!(dx[0]>0.5*scale && dx[1]<2.*scale,"centered derivative scale={scale} far={far} dx={dx:?}");
            // X is exactly linear in u and independent of v. Higher axial
            // partials must contain zero without absorbing the translation.
            let rectangle=[[0.1,0.11],[0.1,0.11]];
            let normal=profile.normal_bounds(rectangle,100).unwrap();assert_eq!(normal.1,1);
            let axial_normal=normal.0.unwrap()[0];
            assert!(axial_normal[0]<=0. && axial_normal[1]>=0.
                && axial_normal[1]-axial_normal[0]<1e-6*scale*scale,
                "centered axial normal scale={scale} far={far} bounds={axial_normal:?}");
            let second=profile.second_derivative_bounds(rectangle,[0,1],100).unwrap();
            assert_eq!(second.1,1);let mixed=second.0.unwrap()[0];
            assert!(mixed[0]<=0. && mixed[1]>=0. && mixed[1]-mixed[0]<scale);
            let higher=profile.higher_derivative_bounds(rectangle,[2,1],100).unwrap();
            assert_eq!(higher.1,1);let axial=higher.0.unwrap()[0];
            assert!(axial[0]<=0. && axial[1]>=0. && axial[1]-axial[0]<scale);

            let bridge=crate::normal_alignment::ProfileSectionBridge::new(
                profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
            let corrected=bridge.constrain_original_rails(100).unwrap().0.unwrap();
            for high in [false,true] {
                let source=[[0.1,0.11],[0.1,0.11]];let target=[[0.11,0.12],[0.6,0.61]];
                let mut workspace=corrected.profile_chord_workspace();
                let fresh=workspace.inspect(source,target,high,1e-4,1000).unwrap();
                assert_eq!(fresh.0,Some(true),"scale={scale} high={high} far={far}");
                assert!(fresh.1<=1000);
                assert_eq!(workspace.inspect(source,target,high,1e-4,1000).unwrap().0,fresh.0);
                assert_eq!(workspace.inspect(source,target,high,1e-4,0).unwrap(),(None,0));
            }
        }
    }
    }
    #[test]
    fn rational_cylinder_radius_covers_whole_corrected_expression() {
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let surface = crate::surface::Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0., 1.]
                .into_iter()
                .map(|x| vec![vec![x, 1., 0.], vec![x, 1., 1.], vec![x, 0., 1.]])
                .collect(),
            weights: vec![vec![1., w, 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 6)
            .unwrap()
            .0
            .unwrap();
        let bridge = crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0, 0.).unwrap(),
            profile.section(0, 1.).unwrap(),
            [1.; 2],
        )
        .unwrap();
        let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let mut shared=corrected.profile_chord_workspace();
        for domain in [[[0.05,0.2],[0.1,0.3]],[[0.25,0.5],[0.4,0.6]],[[0.7,0.95],[0.75,0.9]]] {
            let position=corrected.enclose_with_transverse_correction(domain[0],domain[1],1000).unwrap();
            let jet=corrected.derivatives_with_transverse_correction(domain[0],domain[1],1000).unwrap();
            assert!(position.0.is_some() && jet.0.is_some());
            assert_eq!(shared.enclose_position(domain,1000).unwrap(),position);
            let shared_jet=shared.enclose_derivatives(domain,1000).unwrap();
            assert_eq!(shared_jet.0,jet.0);
            assert!(shared_jet.1<=jet.1,"Retained source hulls must not increase work");
            assert_eq!(shared.enclose_position(domain,1).unwrap(),(position.0,0));
            assert_eq!(shared.enclose_derivatives(domain,1).unwrap(),(jet.0,0));
            let intrinsic=corrected.inspect_self_chords(domain,1e-4,1000).unwrap();
            assert!(intrinsic.0.is_some());
            assert_eq!(shared.inspect_self_chords(domain,1e-4,1000).unwrap(),(intrinsic.0,0));
            assert_eq!(shared.inspect_self_chords(domain,1e-4,0).unwrap(),(None,0));
        }
        for high in [false,true] {
            let a=if high {0.99}else{0.001};let b=if high {0.999}else{0.01};
            let (excluded,spent)=corrected.inspect_profile_endpoint_chord(
                [[a,a+1e-6],[0.,1e-6]],[[0.,1e-6],[b,b+1e-6]],high,1e-4,1000).unwrap();
            assert_eq!(excluded,Some(true));assert!(spent<=1000);
            assert_eq!(corrected.inspect_profile_endpoint_chord(
                [[0.1,0.11],[0.1,0.11]],[[0.11,0.12],[0.6,0.61]],high,1e-4,1000).unwrap().0,Some(true),
                "axial separation >= 0.49 with zero axial normal excludes alignment");
        }
        let centers = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let radius = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let report = corrected
            .inspect_radius(&centers, &radius, 0.1, 10000, 1000000)
            .unwrap();
        assert!(
            report.complete,
            "{} cells={} spans={} pending={}",
            report.reason,
            report.cells,
            report.spans,
            report.pending.len()
        );
        assert!(report.pending.is_empty());
        assert_eq!(report.pending_parameter_fraction(), 0.);
        assert!(report.accepted_error_upper <= 0.1);
        let mut extended = centers.clone();
        extended.control_points[0][0] = -1.;
        extended.control_points[1][0] = 2.;
        let restricted = corrected
            .inspect_radius_on_domains(
                &extended,
                &radius,
                [[1. / 3., 2. / 3.], [0.25, 0.75]],
                0.1,
                10000,
                1000000,
            )
            .unwrap();
        assert!(restricted.complete);
        assert!(restricted.pending.is_empty());
        assert!(corrected
            .inspect_radius_on_domains(&centers, &radius, [[-0.1, 1.], [0., 1.]], 0.1, 100, 10000)
            .is_err());
        assert!(corrected
            .inspect_radius_on_domains(&centers, &radius, [[0.5, 0.5], [0., 1.]], 0.1, 100, 10000)
            .is_err());
        for (cells, spans) in [(0, 1000), (100, 0), (1, 1)] {
            let refused = corrected
                .inspect_radius(&centers, &radius, 0.1, cells, spans)
                .unwrap();
            assert!(!refused.complete);
            assert!(!refused.pending.is_empty());
            assert_eq!(refused.pending_parameter_fraction(), 1.);
        }
        let mut wrong = radius.clone();
        // Radius equals the cylinder at the center but differs at interior
        // quarter parameters. A center-only refutation probe misses this law.
        let off_center_law = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1.2, 0.], vec![0.8, 0.], vec![1.2, 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let off_center = corrected
            .inspect_radius(&centers, &off_center_law, 0.01, 100, 10000)
            .unwrap();
        assert_eq!(off_center.reason, "expression-radius-law-mismatch");
        assert!(!off_center.complete);
        let violation = off_center.violation.as_ref().unwrap();
        assert!(violation.error_bounds[0] > 0.01);
        assert_eq!(off_center.cells, 1);
        assert_eq!(violation.domain[1][0], violation.domain[1][1]);
        assert_ne!(violation.domain[1][0], 0.5);
        for p in &mut wrong.control_points {
            p[0] = 2.;
        }
        let refused = corrected
            .inspect_radius(&centers, &wrong, 0.1, 100, 10000)
            .unwrap();
        assert!(!refused.complete);
        assert!(!refused.pending.is_empty());
        assert_eq!(refused.reason, "expression-radius-law-mismatch");
        let violation = refused.violation.unwrap();
        assert!(violation.error_bounds[0] > 0.1);
        assert!(violation.error_bounds[0] <= 1. && violation.error_bounds[1] >= 1.);
        let jump = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 0.5, 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 0.], vec![2., 0.], vec![2., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(jump.validate().is_err());
        assert!(corrected
            .inspect_radius(&centers, &jump, 0.1, 100, 1000)
            .is_err());
        let (mean, spent) = corrected
            .mean_radius_residual(
                &centers,
                &jump,
                [[0., 1.]; 2],
                [[0., 1.]; 2],
                [[0.; 2]; 3],
                &[I::point(0.); 3],
                I::point(1.),
                1000,
            )
            .unwrap();
        assert!(mean.is_none());
        assert_eq!(spent, 0);
    }
}
