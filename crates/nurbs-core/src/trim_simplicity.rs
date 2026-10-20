//! Sufficient simplicity evidence for a closed rational NURBS UV loop.
//! Failure of a sufficient test is unproven, never evidence of an intersection.
use crate::{Result,check,curve::Curve,curve_distance,curve_jets,trim_domain};
pub struct Pair {pub curves:[usize;2],pub proven:bool}
pub struct Report {
    pub proven_simple: bool,
    pub exact_joins: Option<bool>,
    pub injective: Vec<bool>,
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub cells: usize,
}
/// Failure of a sufficient proof is not proof of self-intersection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    ProvenSimple,
    ClosureMismatch,
    ClosureUnproven,
    InjectivityUnproven,
    PairBudgetExhausted,
    SeparationUnproven,
}
impl Report {
    pub fn outcome(&self) -> Outcome {
        if self.proven_simple {
            Outcome::ProvenSimple
        } else if self.exact_joins == Some(false) {
            Outcome::ClosureMismatch
        } else if self.exact_joins.is_none() {
            Outcome::ClosureUnproven
        } else if self.injective.iter().any(|v| !v) {
            Outcome::InjectivityUnproven
        } else if self.pairs.len() < self.total_pairs {
            Outcome::PairBudgetExhausted
        } else {
            Outcome::SeparationUnproven
        }
    }
}

fn bezier(c: &Curve) -> bool {
    let n = c.control_points.len();
    !c.periodic
        && n == c.degree + 1
        && c.knots[..=c.degree].iter().all(|k| *k == c.knots[c.degree])
        && c.knots[n..].iter().all(|k| *k == c.knots[n])
}
fn projection_delta(
    a: &[f64],
    b: &[f64],
    direction: [f64; 2],
) -> Option<crate::distance_bounds::Interval> {
    use crate::distance_bounds::Interval;
    if a == b {
        return Some(Interval::point(0.));
    }
    let mut result = Interval::point(0.);
    for k in 0..2 {
        result = result
            .add(
                Interval::point(a[k])
                    .sub(Interval::point(b[k]))
                    .ok()?
                    .mul(Interval::point(direction[k]))
                    .ok()?,
            )
            .ok()?;
    }
    Some(result)
}
fn monotone(c: &Curve) -> bool {
    // For rational Bernstein coordinates, the derivative numerator is a sum
    // over i<j of (j-i)(x_j-x_i) w_i w_j B_i B_j / (t(1-t)). All factors
    // except the coordinate differences are positive inside (0,1). Ordered
    // controls and distinct endpoints therefore imply strict monotonicity.
    (0..2).any(|k| {
        let p = &c.control_points;
        (p[0][k] < p[p.len() - 1][k] && p.windows(2).all(|w| w[0][k] <= w[1][k]))
            || (p[0][k] > p[p.len() - 1][k] && p.windows(2).all(|w| w[0][k] >= w[1][k]))
    }) || {
        let a = &c.control_points[0];
        let b = c.control_points.last().unwrap();
        let direction = [b[0] - a[0], b[1] - a[1]];
        projection_delta(b, a, direction).is_some_and(|r| r.lo > 0.)
            && c.control_points
                .windows(2)
                .all(|w| projection_delta(&w[1], &w[0], direction).is_some_and(|r| r.lo >= 0.))
    }
}
// A common strictly signed coordinate derivative over every original knot
// span proves global injectivity of a continuous rational B-spline. Blossom
// extraction encloses the original coefficients; no fitted/decomposed curve
// is used as proof. Exhausting this sufficient test leaves the result unproven.
fn monotone_spans(c:&Curve,cells:&mut usize,max_cells:usize)->Result<bool>{
 if c.periodic{return Ok(false);}
 let spans=(c.degree..c.control_points.len()).filter(|&i|c.knots[i]<c.knots[i+1]).collect::<Vec<_>>();
 for axis in 0..2{for positive in [true,false]{
  let mut pending=spans.iter().map(|&i|(i,[c.knots[i],c.knots[i+1]],0)).collect::<Vec<_>>();
  let mut proven=true;
  while let Some((span,domain,depth))=pending.pop(){
   if *cells==max_cells{return Ok(false);}
   *cells+=1;
   let derivative=curve_jets::enclose(c,span,domain)?[1][axis];
   if if positive{derivative.lo>0.}else{derivative.hi<0.}{continue;}
   if if positive{derivative.hi<=0.}else{derivative.lo>=0.}{proven=false;break;}
   let middle=domain[0]*0.5+domain[1]*0.5;
   if depth==16||middle<=domain[0]||middle>=domain[1]{proven=false;break;}
   pending.push((span,[domain[0],middle],depth+1));pending.push((span,[middle,domain[1]],depth+1));
  }
  if proven{return Ok(true);}
 }}
 Ok(false)
}
fn adjacent_separated(a:&Curve,b:&Curve,join:&[f64])->bool {
    // One open curve lies strictly on one side of a coordinate line through
    // the common endpoint; the other lies on the opposite closed half-plane.
    // Strict positive Bernstein factors exclude every interior return to it.
    (0..2).any(|k| {
        [1., -1.].iter().any(|&sign| {
            let side = |p: &Vec<f64>| {
                if sign > 0. {
                    p[k] > join[k]
                } else {
                    p[k] < join[k]
                }
            };
            let opposite = |p: &Vec<f64>| {
                if sign > 0. {
                    p[k] <= join[k]
                } else {
                    p[k] >= join[k]
                }
            };
            let strict = |c: &Curve| {
                c.control_points
                    .iter()
                    .filter(|p| p.as_slice() != join)
                    .all(side)
                    && c.control_points.iter().any(side)
            };
            (strict(a) && b.control_points.iter().all(opposite))
                || (strict(b) && a.control_points.iter().all(opposite))
        })
    }) || {
        // Arbitrary separating direction, admitted only by outward interval
        // signs. Handles rotated rational arcs without snapping coefficients.
        let (a_control, b_control) = if a.control_points.last().unwrap().as_slice() == join {
            (
                &a.control_points[a.control_points.len() - 2],
                &b.control_points[1],
            )
        } else {
            (
                &b.control_points[b.control_points.len() - 2],
                &a.control_points[1],
            )
        };
        let mut directions = vec![[b_control[0] - a_control[0], b_control[1] - a_control[1]]];
        // Unequal adjacent edge lengths can put the raw control difference
        // on the same side of both curves. Normalize only to propose another
        // separating plane; outward signs below remain the proof.
        let away = |p: &[f64]| {
            let v = [p[0] - join[0], p[1] - join[1]];
            let length = v[0].hypot(v[1]);
            (length.is_finite() && length > 0.).then(|| [v[0] / length, v[1] / length])
        };
        if let (Some(a), Some(b)) = (away(a_control), away(b_control)) {
            directions.push([b[0] - a[0], b[1] - a[1]]);
        }
        directions.into_iter().any(|direction| {
            let sides = |c: &Curve, positive: bool| {
                c.control_points.iter().filter(|p| p.as_slice() != join).all(|p| {
                    projection_delta(p, join, direction).is_some_and(|r| {
                        if positive { r.lo > 0. } else { r.hi < 0. }
                    })
                })
            };
            (sides(a, true) && sides(b, false)) || (sides(a, false) && sides(b, true))
        })
    }
}
// Positive rational Bernstein coefficients prove a strict open-curve side
// even when additional endpoint controls lie on the supporting line.
fn adjacent_supported(a:&Curve,b:&Curve,join:&[f64],second_join:bool,cells:&mut usize,max_cells:usize)->bool {
    use cad_predicates::{AuthoredScalar,Limits,Outcome,PredicateContext,Sign,SourceArena,ToleranceContext};
    if [a,b].iter().any(|c|!bezier(c) || c.degree==0 || c.degree>32 || c.weights.iter().any(|w|*w<=0.)) {return false;}
    let side=|anchor:&[f64],point:&[f64],cells:&mut usize|->Option<Sign> {
        if point==join {return Some(Sign::Zero);}
        if *cells==max_cells {return None;}*cells+=1;
        let raw=[join[0],join[1],anchor[0],anchor[1],point[0],point[1]];
        let arena=SourceArena::authored("original-adjacent-support-line",1,raw.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).ok()?;
        let tolerance=ToleranceContext::default_valid();
        // One charged cell permits at most 256 exact predicate work units.
        let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work:256,..Limits::default()},None);
        match cad_predicates::orient2d(&mut ctx,[arena.leaf(0).ok()?,arena.leaf(1).ok()?],
            [arena.leaf(2).ok()?,arena.leaf(3).ok()?],[arena.leaf(4).ok()?,arena.leaf(5).ok()?]).ok()?.outcome {
            Outcome::Sign(s)=>Some(s),Outcome::Indeterminate(_)=>None,
        }
    };
    for (strict,closed) in [(a,b),(b,a)] {
        let endpoint=if strict.control_points[0].as_slice()==join {strict.control_points.last().unwrap()}
            else if strict.control_points.last().unwrap().as_slice()==join {&strict.control_points[0]}else{return false;};
        for anchor in closed.control_points.iter().filter(|p|p.as_slice()!=join) {
            let Some(sa)=strict.control_points.iter().map(|p|side(anchor,p,cells)).collect::<Option<Vec<_>>>() else {return false;};
            let Some(sb)=closed.control_points.iter().map(|p|side(anchor,p,cells)).collect::<Option<Vec<_>>>() else {return false;};
            for sign in [Sign::Positive,Sign::Negative] {
                let opposite=if sign==Sign::Positive {Sign::Negative}else{Sign::Positive};
                if !sa.iter().all(|s|*s==Sign::Zero || *s==sign) || !sa.contains(&sign)
                    || !sb.iter().all(|s|*s==Sign::Zero || *s==opposite) {continue;}
                let index=if strict.control_points[0].as_slice()==join {sa.len()-1}else{0};
                let far_is_shared=second_join && [closed.control_points.first().unwrap(),closed.control_points.last().unwrap()].contains(&endpoint);
                let far_is_separated=(0..2).any(|axis|closed.control_points.iter().all(|p|endpoint[axis]<p[axis])
                    || closed.control_points.iter().all(|p|endpoint[axis]>p[axis]));
                if sa[index]==sign || far_is_shared || far_is_separated {return true;}
            }
        }
    }
    false
}
pub fn inspect(curves:&[Curve],tolerance_uv:f64,max_pairs:usize,max_cells:usize)->Result<Report>{
    check((2..=256).contains(&curves.len()) && (1..=100000).contains(&max_pairs)
        && (1..=100000).contains(&max_cells) && tolerance_uv.is_finite() && tolerance_uv>0.,
        "Trim simplicity requires 2..256 curves, positive tolerance and bounded work")?;
    let exact_joins=trim_domain::exact_loop_joins(curves)?;
    let mut cells=0;
    let injective=curves.iter().map(|c|if bezier(c)&&monotone(c){Ok(true)}else{monotone_spans(c,&mut cells,max_cells)}).collect::<Result<Vec<_>>>()?;
    let n=curves.len();let mut out=Report{proven_simple:false,exact_joins,injective,pairs:Vec::new(),total_pairs:n*(n-1)/2,cells};
    if exact_joins!=Some(true)||out.injective.iter().any(|v|!*v){return Ok(out);}
    for a in 0..n {for b in a+1..n {
        if out.pairs.len()==max_pairs {return Ok(out);}
        let adjacent=b==a+1 || (a==0&&b==n-1);
        // Two-curve loops may share both owned endpoints; their open images
        // still require strict supporting-line separation.
        let proven=if adjacent {
            let join=if b==a+1 {curves[b].control_points[0].as_slice()}else{curves[0].control_points[0].as_slice()};
            (n>2 && adjacent_separated(&curves[a],&curves[b],join))
                || adjacent_supported(&curves[a],&curves[b],join,n==2,&mut out.cells,max_cells)
        }else if out.cells<max_cells {
            let budget=((max_cells-out.cells)/(out.total_pairs-out.pairs.len())).max(1);
            let r=curve_distance::prove_separation(&curves[a],&curves[b],tolerance_uv,budget)?;
            out.cells+=r.cells;r.distance_interval_mm[0]>0.
        }else{false};
        out.pairs.push(Pair{curves:[a,b],proven});
    }}
    out.proven_simple=out.pairs.iter().all(|p|p.proven);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn polygon(p:&[[f64;2]])->Vec<Curve>{(0..p.len()).map(|i|Curve::from_polyline(vec![p[i].to_vec(),p[(i+1)%p.len()].to_vec()]).unwrap()).collect()}
    #[test]
    fn rational_arc_and_chord_require_owned_endpoints_and_strict_open_side() {
        let arc=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,1.]],weights:vec![1.,0.5,1.],periodic:false};
        let chord=Curve::from_polyline(vec![vec![1.,1.],vec![0.,0.]]).unwrap();
        let two=vec![arc.clone(),chord];let before=format!("{two:?}");
        let proof=inspect(&two,1e-8,100,1000).unwrap();assert!(proof.proven_simple);assert!(proof.cells>0 && proof.cells<=1000);
        assert!(!inspect(&two,1e-8,100,1).unwrap().proven_simple);
        let reversed=two.iter().rev().map(Curve::reverse).collect::<Result<Vec<_>>>().unwrap();assert!(inspect(&reversed,1e-8,100,1000).unwrap().proven_simple);
        let mut rotated=two.clone();for c in &mut rotated {for p in &mut c.control_points {let [x,y]=[p[0],p[1]];p[0]=1024.+x-y;p[1]=-2048.+x+y;}}
        assert!(inspect(&rotated,1e-8,100,1000).unwrap().proven_simple);
        let three=vec![arc.clone(),Curve::from_polyline(vec![vec![1.,1.],vec![0.5,0.5]]).unwrap(),Curve::from_polyline(vec![vec![0.5,0.5],vec![0.,0.]]).unwrap()];
        assert!(inspect(&three,1e-8,100,1000).unwrap().proven_simple);
        let crossed=vec![arc.clone(),Curve::from_polyline(vec![vec![1.,1.],vec![-1.,-1.]]).unwrap(),Curve::from_polyline(vec![vec![-1.,-1.],vec![0.,0.]]).unwrap()];
        assert!(!inspect(&crossed,1e-8,100,1000).unwrap().proven_simple);
        let mut coincident=two.clone();coincident[0].control_points[1]=vec![0.5,0.5];assert!(!inspect(&coincident,1e-8,100,1000).unwrap().proven_simple);
        assert_eq!(format!("{two:?}"),before);
    }
    #[test]
    fn multispan_rational_boundary_preserves_original_definition(){
        let curve=Curve{degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],control_points:vec![vec![0.,0.],vec![0.5,-0.5],vec![1.5,-0.5],vec![2.,0.]],weights:vec![1.,0.8,1.2,1.],periodic:false};
        let mut curves=polygon(&[[0.,0.],[2.,0.],[2.,2.],[0.,2.]]);curves[0]=curve;
        let before=format!("{curves:?}");
        let r=inspect(&curves,1e-8,100,10000).unwrap();assert!(r.proven_simple);assert!(r.cells>0&&r.cells<=10000);
        let reversed=curves.iter().rev().map(Curve::reverse).collect::<Result<Vec<_>>>().unwrap();
        assert!(inspect(&reversed,1e-8,100,10000).unwrap().proven_simple);
        assert_eq!(format!("{curves:?}"),before);
        assert!(!inspect(&curves,1e-8,100,1).unwrap().proven_simple);
    }
    #[test]
    fn multispan_returning_boundary_never_receives_injectivity_proof(){
        let curve=Curve::from_polyline(vec![vec![0.,0.],vec![2.,0.],vec![1.,0.],vec![3.,0.]]).unwrap();
        let mut curves=polygon(&[[0.,0.],[3.,0.],[3.,2.],[0.,2.]]);curves[0]=curve;
        let r=inspect(&curves,1e-8,100,10000).unwrap();assert!(!r.proven_simple);assert!(!r.injective[0]);
    }
    #[test]
    fn square_bowtie_and_incomplete_work_are_distinct() {
        let square = polygon(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        assert!(inspect(&square, 1e-8, 100, 1000).unwrap().proven_simple);
        let crossed = polygon(&[[0., 0.], [1., 1.], [0., 1.], [1., 0.]]);
        assert!(!inspect(&crossed, 1e-8, 100, 1000).unwrap().proven_simple);
        let r = inspect(&square, 1e-8, 1, 1000).unwrap();
        assert!(!r.proven_simple);
        assert_eq!(r.pairs.len(), 1);
        assert_eq!(r.total_pairs, 6);
    }
    #[test]
    fn coincident_edges_and_small_work_never_bypass_pair_checks() {
        let retraced = polygon(&[[0., 0.], [1., 0.], [0., 0.], [0., 1.]]);
        assert!(!inspect(&retraced, 1e-8, 100, 1000).unwrap().proven_simple);
        let square = polygon(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        let r = inspect(&square, 1e-8, 100, 1).unwrap();
        assert!(!r.proven_simple);
        assert!(r.cells <= 1);
        assert_eq!(r.pairs.len(), 6);
        let mut gap = square.clone();
        gap[1].control_points[0][1] = 1e-12;
        let r = inspect(&gap, 1e-6, 100, 1000).unwrap();
        assert!(!r.proven_simple);
        assert_eq!(r.exact_joins, Some(false));
    }
    #[test]
    fn rational_quarter_arcs_form_a_simple_loop() {
        let points = [
            [[1., 0.], [1., 1.], [0., 1.]],
            [[0., 1.], [-1., 1.], [-1., 0.]],
            [[-1., 0.], [-1., -1.], [0., -1.]],
            [[0., -1.], [1., -1.], [1., 0.]],
        ];
        let curves = points
            .iter()
            .map(|p| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.iter().map(|v| v.to_vec()).collect(),
                weights: vec![1., 0.5_f64.sqrt(), 1.],
                periodic: false,
            })
            .collect::<Vec<_>>();
        let before = format!("{curves:?}");
        let r = inspect(&curves, 1e-8, 100, 10000).unwrap();
        assert!(r.proven_simple);
        assert!(r.cells <= 10000);
        assert_eq!(format!("{curves:?}"), before);
    }
    #[test]
    fn rotated_rational_arcs_use_interval_separating_directions() {
        let points = [
            [[1., 0.], [1., 1.], [0., 1.]],
            [[0., 1.], [-1., 1.], [-1., 0.]],
            [[-1., 0.], [-1., -1.], [0., -1.]],
            [[0., -1.], [1., -1.], [1., 0.]],
        ];
        for angle in [0.13_f64, 0.61, 1.17, 2.31] {
            let (sin, cos) = angle.sin_cos();
            let curves = points
                .iter()
                .map(|points| Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: points
                        .iter()
                        .map(|p| vec![cos * p[0] - sin * p[1], sin * p[0] + cos * p[1]])
                        .collect(),
                    weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                    periodic: false,
                })
                .collect::<Vec<_>>();
            assert!(inspect(&curves, 1e-10, 100, 10000).unwrap().proven_simple);
        }
    }
}
