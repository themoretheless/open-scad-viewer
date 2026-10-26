//! Whole-source seam tangent-plane qualification. Positional ownership comes
//! from privately qualified native edge types; angular bounds cover original charts.
use crate::source_shared_edge::SharedEdge;
use nurbs_core::{Error, Result, interval_eval::Interval as I};
pub struct Limits {
    pub max_sine_squared: f64,
    pub cells: usize,
    pub curve_spans: usize,
    pub normal_spans: usize,
}
pub struct Seam<E = SharedEdge> {
    edge: E,
    max_sine_squared: f64,
    bound: [f64; 2],
}
impl<E> Seam<E> {
    pub fn edge(&self) -> &E {
        &self.edge
    }
    pub fn tolerance(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn sine_squared_bounds(&self) -> [f64; 2] {
        self.bound
    }
}
pub struct Report<E = SharedEdge> {
    pub seam: Option<Seam<E>>,
    pub cells: usize,
    pub curve_spans: usize,
    pub normal_spans: usize,
    pub accepted_cells: usize,
    pub uncertain_canonical: Option<[f64; 2]>,
    pub reason: &'static str,
}
mod sealed {
    pub trait Sealed {}
    impl Sealed for crate::source_shared_edge::SharedEdge {}
    impl Sealed for crate::source_projective_edge::Edge {}
}
/// Only freshly qualified native shared-edge types can supply seam ownership.
pub trait Carrier: Clone + sealed::Sealed {
    fn world(&self)->&nurbs_core::curve::Curve;
    fn fragment(&self,side:usize)->&crate::source_boundary_fragment::Fragment;
    fn map_parameter(&self,side:usize,parameter:I)->Result<I>;
    fn inverse_parameter(&self,side:usize,canonical:I)->Result<I>;
    fn literal_endpoints(&self)->bool;
}
impl Carrier for SharedEdge {
    fn world(&self)->&nurbs_core::curve::Curve {SharedEdge::world(self)}
    fn fragment(&self,side:usize)->&crate::source_boundary_fragment::Fragment {&self.uses()[side]}
    fn map_parameter(&self,side:usize,parameter:I)->Result<I> {map(self,side,parameter)}
    fn inverse_parameter(&self,side:usize,canonical:I)->Result<I> {inverse(self,side,canonical)}
    fn literal_endpoints(&self)->bool {self.ranges().is_none()}
}
impl Carrier for crate::source_projective_edge::Edge {
    fn world(&self)->&nurbs_core::curve::Curve {crate::source_projective_edge::Edge::world(self)}
    fn fragment(&self,side:usize)->&crate::source_boundary_fragment::Fragment {self.uses()[side].fragment()}
    fn map_parameter(&self,side:usize,parameter:I)->Result<I> {self.uses()[side].map_normalized(parameter)}
    fn inverse_parameter(&self,side:usize,canonical:I)->Result<I> {
        let d=self.fragment(side).curve().domain();
        self.uses()[side].inverse_normalized_unclipped(canonical)?.intersect(d[0],d[1])
    }
    fn literal_endpoints(&self)->bool {false}
}
pub(crate) fn ranges(edge: &SharedEdge) -> [[[f64; 2]; 2]; 2] {
    edge.ranges().unwrap_or_else(|| {
        std::array::from_fn(|side| {
            // SharedEdge stores traversal orientation, while the map is for the
            // forward original pcurve regardless of fragment traversal.
            let r = edge.reversed()[side] ^ edge.uses()[side].reversed();
            if r {
                [[1., 1.], [0., 1.]]
            } else {
                [[0., 1.], [1., 1.]]
            }
        })
    })
}
pub(crate) fn map(edge: &SharedEdge, side: usize, parameter: I) -> Result<I> {
    let d = edge.uses()[side].curve().domain();
    let range = ranges(edge)[side];
    let first = I::point(range[0][0]).div(I::point(range[0][1]))?;
    let last = I::point(range[1][0]).div(I::point(range[1][1]))?;
    let normalized = parameter
        .sub(I::point(d[0]))?
        .div(I::point(d[1]).sub(I::point(d[0]))?)?;
    first.add(normalized.mul(last.sub(first)?)?)
}
pub(crate) fn inverse(edge: &SharedEdge, side: usize, canonical: I) -> Result<I> {
    let d=edge.uses()[side].curve().domain();
    inverse_unclipped(edge,side,canonical)?.intersect(d[0],d[1])
}
pub(crate) fn inverse_unclipped(edge: &SharedEdge, side: usize, canonical: I) -> Result<I> {
    let d = edge.uses()[side].curve().domain();
    let range = ranges(edge)[side];
    let first = I::point(range[0][0]).div(I::point(range[0][1]))?;
    let last = I::point(range[1][0]).div(I::point(range[1][1]))?;
    let normalized = canonical.sub(first)?.div_signed(last.sub(first)?)?;
    I::point(d[0]).add(normalized.mul(I::point(d[1]).sub(I::point(d[0]))?)?)
}
#[derive(Debug)]
pub struct EndpointLimits {
    pub side: usize,
    pub boundary_end: bool,
    pub limits: nurbs_core::factored_surface_normals::BoundaryLimits,
}
#[derive(Debug)]
pub struct EndpointReport {
    pub canonical: f64,
    pub exact_work: u64,
    pub sides: Vec<EndpointLimits>,
    pub reason: &'static str,
}
/// Diagnose an owned literal carrier endpoint without admitting regular G1.
/// Root-ended/mapped carriers need separate endpoint identity qualification.
pub fn inspect_endpoint_limits(
    edge: &SharedEdge, canonical: f64, max_sine_squared: f64, max_exact_work: u64,
) -> Result<EndpointReport> {
    if ![0.,1.].contains(&canonical) || !max_sine_squared.is_finite()
        || !(0. ..1.).contains(&max_sine_squared)
        || !(1..=100000000).contains(&max_exact_work)
    {
        return Err(Error::new("BREP_SOURCE_SEAM_ENDPOINT", "Choose a carrier endpoint and bounded exact normal work"));
    }
    let mut out = EndpointReport { canonical, exact_work:0, sides:Vec::new(), reason:"source-seam-endpoint-limits-unresolved" };
    if edge.ranges().is_some() {
        out.reason="source-seam-endpoint-limits-identity-required";
        return Ok(out);
    }
    for side in 0..2 {
        let fragment=&edge.uses()[side];
        let curve=fragment.curve();
        let reversed=edge.reversed()[side] ^ fragment.reversed();
        let curve_end=(canonical==1.) ^ reversed;
        let parameter=curve.domain()[usize::from(curve_end)];
        if !fragment.parameter_bounds().contains(&[parameter,parameter]) { continue; }
        let endpoint_knots=if curve_end { &curve.knots[curve.control_points.len()..] }
            else { &curve.knots[..=curve.degree] };
        if !endpoint_knots.iter().all(|k| *k==parameter) { continue; }
        let uv=&curve.control_points[if curve_end { curve.control_points.len()-1 } else {0}];
        let surface=fragment.surface();
        for end in [false,true] {
            let u=surface.knots_u[if end {surface.control_points.len()} else {surface.degree_u}];
            if uv[0]!=u {continue;}
            if out.exact_work==max_exact_work {
                out.reason="source-seam-endpoint-limits-work-limit";
                return Ok(out);
            }
            // Unsupported charts remain unresolved; no substitute sampled normal.
            let r=match nurbs_core::factored_surface_normals::prepare(surface,end,max_exact_work-out.exact_work) {
                Ok(r)=>r,
                Err(_)=>{
                    // Errors expose no consumed-work report: reserve the entire
                    // remaining allowance rather than spend it again elsewhere.
                    out.exact_work=max_exact_work;
                    out.reason="source-seam-endpoint-limits-unsupported";
                    return Ok(out);
                }
            };
            out.exact_work+=r.work;
            if r.reason=="factored-normal-work-limit" {
                out.reason="source-seam-endpoint-limits-work-limit";
                return Ok(out);
            }
            if let Some(proof)=r.certificate {
                out.sides.push(EndpointLimits {side,boundary_end:end,limits:proof.boundary_limits(max_sine_squared)?});
            }
        }
    }
    if out.sides.iter().any(|s|s.limits.distinct_tangent_limits) {
        out.reason="source-seam-endpoint-distinct-tangent-limits";
    }
    Ok(out)
}
/// Exact common-parameter descriptions from private source composition.
pub trait SynchronizedCarrier:Carrier {
    fn synchronized_curve(&self,side:usize)->Option<(&nurbs_core::curve::Curve,bool,[f64;2])>;
}
impl SynchronizedCarrier for SharedEdge {
    fn synchronized_curve(&self,side:usize)->Option<(&nurbs_core::curve::Curve,bool,[f64;2])> {
        if self.ranges().is_some() {return None;}
        let f=&self.uses()[side];Some((f.curve(),self.reversed()[side]^f.reversed(),[1.,1.]))
    }
}
impl SynchronizedCarrier for crate::source_projective_edge::Edge {
    fn synchronized_curve(&self,side:usize)->Option<(&nurbs_core::curve::Curve,bool,[f64;2])> {
        let u=&self.uses()[side];let r=u.range();let weights=u.weights();
        let endpoint=|q:[f64;2]|q[0]==0. || q[0]==q[1];
        if !r.into_iter().all(endpoint) {return None;}
        Some((u.fragment().curve(),u.reversed()^u.fragment().reversed(),weights))
    }
}
pub struct SynchronizedReport<E=SharedEdge> {
    pub report:Report<E>,
    pub polynomial_work:usize,
    pub synchronized_reason:&'static str,
}
/// Fresh shared-parameter polynomial angle bound, followed by the existing
/// conservative interval path when the supported representation is insufficient.
/// Polynomial work has a separate explicit budget; original roots are retained.
pub fn qualify_with_synchronized_boundaries<E:SynchronizedCarrier>(edge:&E,limits:Limits,max_work:usize)->Result<SynchronizedReport<E>> {
    validate_limits(&limits)?;
    if !(1..=1000000).contains(&max_work) {return Err(Error::new("BREP_SOURCE_SEAM_TANGENCY","Bound synchronized polynomial work"));}
    let mut work=0;let mut reason="synchronized-source-map-unsupported";
    if let (Some((a,ar,aw)),Some((b,br,bw)))=(edge.synchronized_curve(0),edge.synchronized_curve(1)) {
        if limits.curve_spans>=2 && limits.normal_spans>=2 {
            let proof=nurbs_core::synchronized_boundary_normals::inspect_projective([edge.fragment(0).surface(),edge.fragment(1).surface()],[a,b],[ar,br],[aw,bw],limits.max_sine_squared,max_work)?;
            work=proof.work;reason=proof.reason;
            if let Some(aligned)=proof.aligned {
                let bounds=proof.sine_squared_bounds.unwrap();
                let report=Report{seam:aligned.then(||Seam{edge:edge.clone(),max_sine_squared:limits.max_sine_squared,bound:bounds}),
                    cells:1,curve_spans:2,normal_spans:2,accepted_cells:usize::from(aligned),uncertain_canonical:None,
                    reason:if aligned {"source-seam-synchronized-tangent-planes-qualified"}else{"source-seam-angular-envelope-oblique"}};
                return Ok(SynchronizedReport{report,polynomial_work:work,synchronized_reason:reason});
            }
        }
    }
    Ok(SynchronizedReport{report:qualify(edge,limits)?,polynomial_work:work,synchronized_reason:reason})
}
fn validate_limits(limits:&Limits)->Result<()> {
    if !limits.max_sine_squared.is_finite() || !(0. ..1.).contains(&limits.max_sine_squared)
        || !(1..=100000).contains(&limits.cells) || !(1..=100000).contains(&limits.curve_spans)
        || !(1..=100000).contains(&limits.normal_spans) {
        return Err(Error::new("BREP_SOURCE_SEAM_TANGENCY","Choose finite angular tolerance and bounded seam work"));
    }Ok(())
}
/// Every accepted interval bounds all corresponding normal pairs, including
/// endpoint root enclosures and both incident original knot sides. Antiparallel
/// normals are compatible tangent planes; orientation is checked by the shell.
/// This certifies tolerance-based G1 only, not curvature/G2 or fillet radius.
pub fn qualify<E: Carrier>(edge: &E, limits: Limits) -> Result<Report<E>> {
    validate_limits(&limits)?;
    let ends = edge.fragment(0).parameter_bounds();
    let lo = ends.iter().map(|r| r[0]).fold(f64::INFINITY, f64::min);
    let hi = ends.iter().map(|r| r[1]).fold(f64::NEG_INFINITY, f64::max);
    let coverage = edge.map_parameter(0, I::new(lo, hi)?)?.intersect(0., 1.)?;
    let mut queue = vec![[coverage.lo, coverage.hi]];
    // Only literal full-carrier endpoints have exact canonical representatives.
    // Root enclosures and affine ranges retain whole-interval qualification.
    if edge.literal_endpoints()
        && (0..2).all(|side| {
            edge.fragment(side)
                .parameter_bounds()
                .iter()
                .all(|r| r[0] == r[1])
        })
    {
        queue.push([coverage.hi, coverage.hi]);
        queue.push([coverage.lo, coverage.lo]);
    }
    let mut out = Report {
        seam: None,
        cells: 0,
        curve_spans: 0,
        normal_spans: 0,
        accepted_cells: 0,
        uncertain_canonical: None,
        reason: "source-seam-work-limit",
    };
    let mut bound = [1_f64, 0_f64];
    while let Some(q) = queue.pop() {
        out.uncertain_canonical = Some(q);
        if out.cells == limits.cells || out.normal_spans == limits.normal_spans {
            return Ok(out);
        }
        out.cells += 1;
        let mut rectangles = [[[0.; 2]; 2]; 2];
        for side in 0..2 {
            let fragment = edge.fragment(side);
            let c = fragment.curve();
            let t = edge.inverse_parameter(side, I::new(q[0], q[1])?)?;
            let count = (c.degree..c.control_points.len())
                .filter(|&i| {
                    c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
                })
                .count();
            if count > limits.curve_spans - out.curve_spans {
                return Ok(out);
            }
            out.curve_spans += count;
            let uv = nurbs_core::interval_eval::evaluate_interval(c, t)?;
            let s = fragment.surface();
            let chart = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            for axis in 0..2 {
                // Fragment admission proves the actual source path lies in
                // this chart; intersecting an outward enclosure is sound.
                let b = uv[axis].intersect(chart[axis][0], chart[axis][1])?;
                rectangles[side][axis] = [b.lo, b.hi];
            }
        }
        let r = nurbs_core::normal_alignment::inspect_pair(
            [edge.fragment(0).surface(), edge.fragment(1).surface()],
            rectangles,
            limits.max_sine_squared,
            limits.normal_spans - out.normal_spans,
        )?;
        out.normal_spans += r.spans;
        if r.aligned == Some(true) {
            let b = r.sine_squared_interval.unwrap();
            bound[0] = bound[0].min(b[0]);
            bound[1] = bound[1].max(b[1]);
            out.accepted_cells += 1;
            continue;
        }
        if r.aligned == Some(false) {
            out.reason = "source-seam-angular-envelope-oblique";
            return Ok(out);
        }
        if r.reason == "span-limit" {
            return Ok(out);
        }
        if q[0] == q[1] {
            out.reason = "source-seam-endpoint-normal-unresolved";
            return Ok(out);
        }
        let mid = q[0] * 0.5 + q[1] * 0.5;
        if !(q[0] < mid && mid < q[1]) {
            out.reason = "source-seam-resolution-limit";
            return Ok(out);
        }
        queue.push([mid, q[1]]);
        queue.push([q[0], mid]);
    }
    out.seam = Some(Seam {
        edge: edge.clone(),
        max_sine_squared: limits.max_sine_squared,
        bound,
    });
    out.uncertain_canonical = None;
    out.reason = "source-seam-tangent-planes-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
    use nurbs_core::{curve::Curve, surface::Surface};
    #[test]
    fn projective_rational_shared_boundary_tangency_survives_original_definition_replay() {
        let make=|zs:[f64;2],factors:[f64;3]|Surface {
            degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:zs.into_iter().map(|z|vec![vec![1.,0.,z],vec![1.,1.,z],vec![0.,1.,z]]).collect(),
            weights:vec![factors.to_vec();2],periodic_u:false,periodic_v:false,
        };
        // Dyadic weights keep the authored projective composition exact.
        // A general rational arc is sufficient; no conic recognition is used.
        let a=make([0.,1.],[1.,0.5,1.]);
        let b=make([1.,2.],[1.,1.,4.]);
        let pa=Curve::from_polyline(vec![vec![1.,0.],vec![1.,1.]]).unwrap();
        let pb=Curve::from_polyline(vec![vec![0.,0.],vec![0.,1.]]).unwrap();
        let world=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:a.control_points[1].clone(),weights:a.weights[1].clone(),periodic:false};
        let fa=Fragment::new(&a,&pa,Endpoint::Parameter(0.),Endpoint::Parameter(1.)).unwrap();
        let fb=Fragment::new(&b,&pb,Endpoint::Parameter(1.),Endpoint::Parameter(0.)).unwrap();
        let ranges=[[[0.,1.],[1.,1.]];2];
        let r=crate::source_projective_edge::qualify(&world,[&fa,&fb],ranges,[[1.,1.],[1.,2.]],
            [None;2],[[None;2];2],100000000,10000).unwrap();
        let edge=r.edge.expect(r.reason);
        let definition=edge.definition();
        let encoded=value_codec::encode_binary(&definition).unwrap();
        let restored=crate::source_projective_edge::restore(value_codec::decode_binary(&encoded).unwrap(),10000,100000000,10000).unwrap();
        let restored=restored.edge.expect(restored.reason);
        assert_eq!(restored.definition(),definition);
        for source in [&edge,&restored] {
            let proof=qualify_with_synchronized_boundaries(source,
                Limits{max_sine_squared:1e-10,cells:1,curve_spans:2,normal_spans:2},100000).unwrap();
            assert_eq!(proof.report.reason,"source-seam-synchronized-tangent-planes-qualified");
            assert!(proof.polynomial_work>0 && proof.polynomial_work<=100000);
            let seam=proof.report.seam.unwrap();
            assert_eq!(seam.edge().definition(),definition);
            assert!(seam.sine_squared_bounds()[1]<=1e-10);
        }
        let cutter_a=Curve::from_polyline(vec![vec![-1.,0.5],vec![2.,0.5]]).unwrap();
        let cutter_b=Curve::from_polyline(vec![vec![-1.,0.],vec![2.,1.]]).unwrap();
        let root_a=crate::source_contact_point::qualify(&a,&pa,&cutter_a,[[0.4,0.6],[0.6,0.75]],10000).unwrap();
        let root_a=root_a.point.expect(root_a.reason);
        let root_b=crate::source_contact_point::qualify(&b,&pb,&cutter_b,[[0.3,0.4],[0.3,0.4]],10000).unwrap();
        let root_b=root_b.point.expect(root_b.reason);
        let trimmed_a=Fragment::new(&a,&pa,Endpoint::Parameter(0.),Endpoint::Crossing{point:root_a,role:Role::Boundary}).unwrap();
        let trimmed_b=Fragment::new(&b,&pb,Endpoint::Crossing{point:root_b,role:Role::Boundary},Endpoint::Parameter(0.)).unwrap();
        // q=1/2 lies on x=y. The second original parameter is 1/3,
        // so a rounded binary64 candidate must not identify the root.
        let root_plane=[[0.,0.,0.],[1.,1.,0.],[0.,0.,1.]];
        let trimmed=crate::source_projective_edge::qualify(&world,[&trimmed_a,&trimmed_b],ranges,[[1.,1.],[1.,2.]],
            [None,Some(root_plane)],[[None;2];2],100000000,10000).unwrap();
        let trimmed=trimmed.edge.expect(trimmed.reason);
        let trimmed_definition=trimmed.definition();
        let restored=crate::source_projective_edge::restore(trimmed_definition.clone(),10000,100000000,10000).unwrap();
        let restored=restored.edge.expect(restored.reason);
        assert_eq!(restored.definition(),trimmed_definition);
        for source in [&trimmed,&restored] {
            let proof=qualify_with_synchronized_boundaries(source,
                Limits{max_sine_squared:1e-10,cells:1,curve_spans:2,normal_spans:2},100000).unwrap();
            assert!(proof.report.seam.is_some());
        }
        let mut wrong=definition;
        wrong["uses"][1]["weights"]=value_codec::json!([1.,1.]);
        assert!(crate::source_projective_edge::restore(wrong,10000,100000000,10000).unwrap().edge.is_none());
    }

    fn limits() -> Limits {
        Limits {
            max_sine_squared: 1e-3,
            cells: 10000,
            curve_spans: 20000,
            normal_spans: 20000,
        }
    }
    #[test]
    fn owned_endpoint_limits_preserve_sharp_tip_and_exact_work_budget() {
        let s=Surface {
            degree_u:2, degree_v:2,
            knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.];3],vec![vec![0.5,0.,0.];3],
                vec![vec![1.,0.,0.],vec![1.,0.5,0.],vec![1.,1.,1.]]],
            weights:vec![vec![1.;3];3],periodic_u:false,periodic_v:false,
        };
        let pc=Curve::from_polyline(vec![vec![0.,0.],vec![1.,0.]]).unwrap();
        let world=Curve::from_polyline(vec![vec![0.,0.,0.],vec![1.,0.,0.]]).unwrap();
        let a=Fragment::new(&s,&pc,Endpoint::Parameter(0.),Endpoint::Parameter(1.)).unwrap();
        let b=Fragment::new(&s,&pc,Endpoint::Parameter(1.),Endpoint::Parameter(0.)).unwrap();
        let edge=crate::source_shared_edge::qualify(&world,[&a,&b],[false,false],100000000).unwrap().edge.unwrap();
        let r=inspect_endpoint_limits(&edge,0.,1e-6,100000000).unwrap();
        assert_eq!(r.reason,"source-seam-endpoint-distinct-tangent-limits");
        assert_eq!(r.sides.len(),2);
        assert!(r.exact_work<=100000000);
        assert!(r.sides.iter().all(|s|s.limits.collapsed_point==Some([0.;3]) && s.limits.distinct_tangent_limits));
        let stopped=inspect_endpoint_limits(&edge,0.,1e-6,1).unwrap();
        assert!(stopped.sides.is_empty());assert!(stopped.exact_work<=1);
        assert_eq!(stopped.reason,"source-seam-endpoint-limits-work-limit");
        assert!(qualify(&edge,limits()).unwrap().seam.is_none());
        let opposite=inspect_endpoint_limits(&edge,1.,1e-6,100000000).unwrap();
        assert!(opposite.sides.iter().all(|s|!s.limits.distinct_tangent_limits));
        assert!(inspect_endpoint_limits(&edge,0.5,1e-6,100).is_err());
        assert!(inspect_endpoint_limits(&edge,0.,1e-6,0).is_err());
        let mut reverse_world=world.clone();reverse_world.control_points.reverse();
        let reverse=crate::source_shared_edge::qualify(&reverse_world,[&a,&b],[true,true],100000000).unwrap().edge.unwrap();
        assert_eq!(inspect_endpoint_limits(&reverse,1.,1e-6,100000000).unwrap().reason,
            "source-seam-endpoint-distinct-tangent-limits");
        let carrier=Curve::from_polyline(vec![vec![-1.,0.,0.],vec![2.,0.,0.]]).unwrap();
        let mapped=crate::source_mapped_edge::qualify(&carrier,[&a,&b],
            [[[1.,3.],[2.,3.]],[[1.,3.],[2.,3.]]],[None;2],100000000,0).unwrap().edge.unwrap();
        let rejected=inspect_endpoint_limits(&mapped,0.,1e-6,100000000).unwrap();
        assert_eq!(rejected.reason,"source-seam-endpoint-limits-identity-required");
        assert!(rejected.sides.is_empty());assert_eq!(rejected.exact_work,0);
    }
    fn patch(crease: bool) -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|u| {
                    (0..3)
                        .map(|v| {
                            vec![
                                u as f64 / 2.,
                                v as f64 / 2.,
                                f64::from(u == 2)
                                    + f64::from(v == 2)
                                    + if crease { v as f64 / 2. } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn shared(crease: bool) -> SharedEdge {
        let mut a = patch(false);
        for row in &mut a.control_points {
            for (v, p) in row.iter_mut().enumerate() {
                p[2] -= f64::from(v == 2);
            }
        }
        let b = patch(crease);
        let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 0., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let fa = Fragment::new(&a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        let fb = Fragment::new(&b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
        crate::source_shared_edge::qualify(&world, [&fa, &fb], [false, false], 100_000_000)
            .unwrap()
            .edge
            .unwrap()
    }
    #[test]
    fn rotating_tangent_planes_are_certified_over_the_complete_curved_seam() {
        let edge = shared(false);
        let before = edge.uses()[0].definition();
        let r = qualify(&edge, limits()).unwrap();
        assert!(
            r.seam.is_some(),
            "{} {:?} cells {}",
            r.reason,
            r.uncertain_canonical,
            r.cells
        );
        assert!(r.accepted_cells > 1);
        let seam = r.seam.unwrap();
        assert!(seam.sine_squared_bounds()[1] <= 1e-3);
        assert_eq!(seam.edge().uses()[0].definition(), before);
        let mut short = limits();
        short.cells = 1;
        let r = qualify(&edge, short).unwrap();
        assert!(r.seam.is_none());
        assert!(r.uncertain_canonical.is_some());
        let mut short = limits();
        short.curve_spans = 1;
        let r = qualify(&edge, short).unwrap();
        assert!(r.seam.is_none());
        assert_eq!(r.curve_spans, 1);
        let mut short = limits();
        short.normal_spans = 1;
        let r = qualify(&edge, short).unwrap();
        assert!(r.seam.is_none());
        assert_eq!(r.normal_spans, 1);
    }
    #[test]
    fn positional_coincidence_does_not_hide_a_crease() {
        let r = qualify(&shared(true), limits()).unwrap();
        assert!(r.seam.is_none());
        assert!(r.uncertain_canonical.is_some());
    }
    #[test]
    fn matching_endpoint_planes_do_not_admit_an_interior_crease() {
        // z = 2 * amplitude * u * (1-u) * v: the seam and endpoint
        // tangent planes coincide, but the tangent plane tilts inside it.
        for amplitude in [0.1, 1., 10.] {
            let a = Surface {
                degree_u: 2, degree_v: 1,
                knots_u: vec![0., 0., 0., 1., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: (0..3).map(|i| (0..2).map(|j|
                    vec![i as f64 * 0.5, j as f64, 0.]).collect()).collect(),
                weights: vec![vec![1.; 2]; 3],
                periodic_u: false, periodic_v: false,
            };
            let mut b = a.clone();
            b.control_points[1][1][2] = amplitude;
            let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
            let carrier = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
            let fa = Fragment::new(&a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
            let fb = Fragment::new(&b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
            let edge = crate::source_shared_edge::qualify(&carrier, [&fa, &fb], [false, false], 100_000_000)
                .unwrap().edge.unwrap();
            for endpoint in [0., 1.] {
                let point = [[endpoint, endpoint], [0., 0.]];
                let aligned = nurbs_core::normal_alignment::inspect_pair(
                    [&a, &b], [point, point], 1e-3, 1000).unwrap();
                assert_eq!(aligned.aligned, Some(true));
            }
            let midpoint = [[0.5, 0.5], [0., 0.]];
            let tilted = nurbs_core::normal_alignment::inspect_pair(
                [&a, &b], [midpoint, midpoint], 1e-3, 1000).unwrap();
            assert_eq!(tilted.aligned, Some(false));
            let report = qualify(&edge, limits()).unwrap();
            assert!(report.seam.is_none(), "Interior crease admitted: {amplitude}");
            assert!(report.uncertain_canonical.is_some());
        }
    }
    #[test]
    fn rational_root_ends_keep_their_source_definitions() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., -0.5, 0.], vec![0., 0.5, 0.]],
                vec![vec![1., -0.5, 0.], vec![1., 0.5, 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut pc = Curve::from_polyline(vec![vec![-0.25, 0.5], vec![1.25, 0.5]]).unwrap();
        pc.weights[1] = 0.75;
        let mut world =
            Curve::from_polyline(vec![vec![-0.25, 0., 0.], vec![1.25, 0., 0.]]).unwrap();
        world.weights = pc.weights.clone();
        let mut ends = Vec::new();
        for (u, selector) in [(0.125, [0.2, 0.4]), (0.875, [0.7, 0.9])] {
            let boundary = Curve::from_polyline(vec![vec![u, 0.], vec![u, 1.]]).unwrap();
            let point = crate::source_contact_point::qualify(
                &s,
                &boundary,
                &pc,
                [[0.4, 0.6], selector],
                16,
            )
            .unwrap()
            .point
            .unwrap();
            ends.push(Endpoint::Crossing {
                point,
                role: Role::Contact,
            });
        }
        let a = Fragment::new(&s, &pc, ends[0].clone(), ends[1].clone()).unwrap();
        let b = Fragment::new(&s, &pc, ends[1].clone(), ends[0].clone()).unwrap();
        let edge =
            crate::source_shared_edge::qualify(&world, [&a, &b], [false, false], 100_000_000)
                .unwrap()
                .edge
                .unwrap();
        let before = [a.definition(), b.definition()];
        let r = qualify(&edge, limits()).unwrap();
        assert!(r.seam.is_some(), "{}", r.reason);
        let seam = r.seam.unwrap();
        assert_eq!(seam.edge().uses()[0].definition(), before[0]);
        assert_eq!(seam.edge().uses()[1].definition(), before[1]);
    }
    #[test]
    fn reversed_original_curve_and_affine_partial_carriers_preserve_correspondence() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., -0.5, 0.], vec![0., 0.5, 0.]],
                vec![vec![1., -0.5, 0.], vec![1., 0.5, 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
        let mut b = Curve::from_polyline(vec![vec![1., 0.5], vec![0., 0.5]]).unwrap();
        b.knots = vec![10., 10., 18., 18.];
        let fa = Fragment::new(&s, &a, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        let fb = Fragment::new(&s, &b, Endpoint::Parameter(10.), Endpoint::Parameter(18.)).unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let full =
            crate::source_shared_edge::qualify(&world, [&fa, &fb], [false, true], 100_000_000)
                .unwrap()
                .edge
                .unwrap();
        assert!(qualify(&full, limits()).unwrap().seam.is_some());
        let carrier = Curve::from_polyline(vec![vec![-1., 0., 0.], vec![2., 0., 0.]]).unwrap();
        let ranges = [[[1., 3.], [2., 3.]], [[2., 3.], [1., 3.]]];
        let partial = crate::source_mapped_edge::qualify(
            &carrier,
            [&fa, &fb],
            ranges,
            [None; 2],
            100_000_000,
            0,
        )
        .unwrap()
        .edge
        .unwrap();
        let report = qualify(&partial, limits()).unwrap();
        assert!(report.seam.is_some(), "{}", report.reason);
        assert_eq!(report.seam.unwrap().edge().ranges(), Some(ranges));
    }
    #[test]
    fn singular_endpoint_is_not_a_regular_tangent_plane() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let a = Fragment::new(&s, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
        let b = Fragment::new(&s, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
        let edge =
            crate::source_shared_edge::qualify(&world, [&a, &b], [false, false], 100_000_000)
                .unwrap()
                .edge
                .unwrap();
        let mut budget = limits();
        budget.cells = 32;
        let report = qualify(&edge, budget).unwrap();
        assert!(report.seam.is_none());
        assert_eq!(report.reason, "source-seam-endpoint-normal-unresolved");
        assert_eq!(report.cells, 1);
        let uncertain = report.uncertain_canonical.unwrap();
        assert!(uncertain[0] <= 0. && uncertain[1] >= 0.);
    }
}
