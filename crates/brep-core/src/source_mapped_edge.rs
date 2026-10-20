//! Fresh shared ownership for differently covered original source curves.
use crate::{
    source_affine_use::{self, MappedUse},
    source_boundary_fragment::{Endpoint, Fragment, Role},
    source_contact_point::SourcePoint,
    source_shared_edge::{self, Report, SharedEdge},
};
use cad_predicates::{
    AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
};
use nurbs_core::{curve::Curve, Error, Result};
type Range = [[f64; 2]; 2];
fn error() -> Error {
    Error::new(
        "BREP_SOURCE_MAPPED_EDGE",
        "Invalid exact mapped endpoint input",
    )
}
fn fixed_identity(
    parameters: [[f64; 3]; 2],
    ranges: [Range; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    if out.work_used == max_work {
        return Ok(false);
    }
    let values = parameters
        .iter()
        .flatten()
        .chain(ranges.iter().flatten().flatten())
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("mapped-fixed-endpoints", 1, values).map_err(|_| error())?;
    let pp = std::array::from_fn(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()));
    let ranges = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| source.leaf(6 + 4 * i + 2 * j + k).unwrap())
        })
    });
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::affine_parameter_identity(&mut ctx, pp, ranges).map_err(|_| error())?;
    out.work_used += r.work_used;
    Ok(r.outcome == ParameterIdentity::Equal)
}
fn linear_identity(
    points: [(&SourcePoint, Role); 2],
    ranges: [Range; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    let line = |c: &Curve| {
        let d = c.domain();
        c.degree == 1
            && c.control_points.len() == 2
            && c.knots[..2].iter().all(|&t| t == d[0])
            && c.knots[2..].iter().all(|&t| t == d[1])
    };
    let mut values = Vec::new();
    for (point, role) in points {
        let (main, other) = match role {
            Role::Boundary => (point.boundary(), point.contact()),
            Role::Contact => (point.contact(), point.boundary()),
        };
        if !line(main) || !line(other) {
            return Ok(false);
        }
        for (p, w) in main.control_points.iter().zip(&main.weights) {
            values.extend(p.iter().copied());
            values.push(*w);
        }
        for p in &other.control_points {
            values.extend(p.iter().copied());
        }
    }
    if out.work_used == max_work {
        return Ok(false);
    }
    values.extend(ranges.iter().flatten().flatten().copied());
    let source = SourceArena::authored(
        "mapped-linear-roots",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| error())?;
    let main = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| source.leaf(10 * i + 3 * j + k).unwrap()))
    });
    let cutter = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| source.leaf(10 * i + 6 + 2 * j + k).unwrap())
        })
    });
    let ranges = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| source.leaf(20 + 4 * i + 2 * j + k).unwrap())
        })
    });
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::line_crossing_parameter_identity_affine(&mut ctx, main, cutter, ranges)
        .map_err(|_| error())?;
    out.work_used += r.work_used;
    Ok(r.outcome == ParameterIdentity::Equal)
}
fn mixed_identity(
    world: &Curve,
    point: &SourcePoint,
    role: Role,
    parameter: f64,
    fixed: &MappedUse,
    domain: [f64;2],
    plane: [[f64; 3]; 3],
    out: &mut Report,
    max_work: u64,
    max_driver: usize,
) -> Result<bool> {
    out.root_checks += 1;
    if !source_shared_edge::source_point_cut_plane(point, role, plane, out, max_work)?
        || out.work_used == max_work
    {
        return Ok(false);
    }
    let d = fixed.fragment().curve().domain();
    let Some(r) = nurbs_core::curve_surface_plane::verify_curve_point_affine(
        world,
        plane,
        [parameter, d[0], d[1]],
        fixed.range(),
        (max_work - out.work_used).min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(false);
    };
    out.work_used += r.work_used;
    if r.outcome != cad_predicates::BezierIdentity::Equal {
        return Ok(false);
    }
    if source_shared_edge::unique_world_plane_domain(world, plane, Some(domain), out, max_work, max_driver)? {
        return Ok(true);
    }
    // The global original-coordinate proof also handles a zero derivative
    // at an endpoint. Keep that proof under the same remaining work budget.
    source_shared_edge::unique_world_plane(world, plane, out, max_work, max_driver)
}
/// Full source compositions are freshly verified for both affine maps before
/// matching their actual retained ends. Scalar fractions are compared exactly;
/// nonlinear roots require a proven unique canonical world-plane intersection.
pub fn qualify(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [Range; 2],
    planes: [Option<[[f64; 3]; 3]>; 2],
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    qualify_with_candidates(
        world,
        fragments,
        ranges,
        planes,
        [[None; 2]; 2],
        max_work,
        max_driver,
    )
}
/// Raw exact parameter candidates indexed by use and directed endpoint. Each
/// root candidate is rechecked against its original UV equations and selector.
pub fn qualify_with_candidates(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [Range; 2],
    planes: [Option<[[f64; 3]; 3]>; 2],
    candidates: [[Option<[f64; 2]>; 2]; 2],
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    if !(1..=100_000_000).contains(&max_work) || max_driver > 100000 {
        return Err(error());
    }
    let mut out = Report {
        edge: None,
        work_used: 0,
        root_checks: 0,
        driver_cells: 0,
        reason: "mapped-source-composition-unproven",
    };
    let mut mapped = Vec::new();
    for i in 0..2 {
        if out.work_used == max_work {
            return Ok(out);
        }
        let r =
            source_affine_use::qualify(world, fragments[i], ranges[i], max_work - out.work_used)?;
        out.work_used += r.exact_work;
        let Some(m) = r.mapped else {
            out.reason = r.reason;
            return Ok(out);
        };
        mapped.push(m);
    }
    let uses = [mapped.remove(0), mapped.remove(0)];
    if uses[0].reversed() == uses[1].reversed() {
        out.reason = "mapped-source-uses-not-opposite";
        return Ok(out);
    }
    out.reason = "mapped-source-endpoints-unproven";
    for i in 0..2 {
        let mut authored = [None; 2];
        for use_index in 0..2 {
            let endpoint_index = if use_index == 0 { i } else { 1 - i };
            authored[use_index] = match &fragments[use_index].endpoints()[endpoint_index] {
                Endpoint::Carrier { .. } => None,
                Endpoint::Parameter(t) => Some(*t),
                Endpoint::Crossing { point, role } => {
                    if let Some(candidate) = candidates[use_index][endpoint_index] {
                        if out.work_used == max_work {
                            return Ok(out);
                        }
                        let r = crate::source_root_parameter::verify(
                            point,
                            candidate,
                            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
                        )?;
                        out.work_used += r.work_used;
                        out.root_checks += 1;
                        r.parameters.map(|p| {
                            p[match role {
                                Role::Boundary => 0,
                                Role::Contact => 1,
                            }]
                        })
                    } else {
                        None
                    }
                }
            };
        }
        let same = if let [Some(a), Some(b)] = authored {
            let da = fragments[0].curve().domain();
            let db = fragments[1].curve().domain();
            fixed_identity(
                [[a, da[0], da[1]], [b, db[0], db[1]]],
                ranges,
                &mut out,
                max_work,
            )?
        } else {
            match (
                &fragments[0].endpoints()[i],
                &fragments[1].endpoints()[1 - i],
            ) {
                (Endpoint::Carrier { .. }, _) | (_, Endpoint::Carrier { .. }) =>
                    crate::source_carrier_point::same_mapped_endpoint(world, fragments, ranges,
                        [&fragments[0].endpoints()[i], &fragments[1].endpoints()[1-i]]),
                (Endpoint::Parameter(a), Endpoint::Parameter(b)) => {
                    let da = fragments[0].curve().domain();
                    let db = fragments[1].curve().domain();
                    fixed_identity(
                        [[*a, da[0], da[1]], [*b, db[0], db[1]]],
                        ranges,
                        &mut out,
                        max_work,
                    )?
                }
                (
                    Endpoint::Crossing { point: a, role: ar },
                    Endpoint::Crossing { point: b, role: br },
                ) => {
                    if linear_identity([(a, *ar), (b, *br)], ranges, &mut out, max_work)? {
                        true
                    } else if let Some(plane) = planes[i] {
                        // Both original root enclosures are mapped outward
                        // into the same canonical parameter domain. Uniqueness
                        // is required on their entire contiguous union.
                        let aa=uses[0].parameter_bounds()?[i];
                        let bb=uses[1].parameter_bounds()?[1-i];
                        let domain=[aa[0].min(bb[0]),aa[1].max(bb[1])];
                        source_shared_edge::common_plane_root_in_domain(
                            world,plane,[(a,*ar),(b,*br)],domain,&mut out,max_work,max_driver,
                        )?
                    } else {
                        false
                    }
                }
                (Endpoint::Crossing { point, role }, Endpoint::Parameter(t)) => {
                    if let Some(plane) = planes[i] {
                        mixed_identity(
                            world, point, *role, *t, &uses[1], {
                                let aa=uses[0].parameter_bounds()?[i];
                                let bb=uses[1].parameter_bounds()?[1-i];
                                [aa[0].min(bb[0]),aa[1].max(bb[1])]
                            }, plane, &mut out, max_work,
                            max_driver,
                        )?
                    } else {
                        false
                    }
                }
                (Endpoint::Parameter(t), Endpoint::Crossing { point, role }) => {
                    if let Some(plane) = planes[i] {
                        mixed_identity(
                            world, point, *role, *t, &uses[0], {
                                let aa=uses[0].parameter_bounds()?[i];
                                let bb=uses[1].parameter_bounds()?[1-i];
                                [aa[0].min(bb[0]),aa[1].max(bb[1])]
                            }, plane, &mut out, max_work,
                            max_driver,
                        )?
                    } else {
                        false
                    }
                }
            }
        };
        if !same {
            return Ok(out);
        }
    }
    out.edge = Some(SharedEdge::from_mapped(uses, planes, candidates));
    out.reason = "mapped-source-shared-edge-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    fn fixture() -> (Surface, Surface, Curve, Curve, Curve) {
        let surface = |vertical: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if vertical {
                                vec![u as f64, 0., v as f64]
                            } else {
                                vec![u as f64, v as f64, 0.]
                            }
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let a = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 4.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let b = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![0.25, 0.], vec![0.75, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let world = Curve {
            degree: 1,
            knots: vec![-5., -5., 3., 3.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        (surface(false), surface(true), a, b, world)
    }
    const RANGES: [Range; 2] = [[[0., 1.], [1., 1.]], [[1., 4.], [3., 4.]]];
    #[test]
    fn full_affine_composition_does_not_make_a_retained_fragment_complete() {
        let (sa,sb,a,mut b,world)=fixture();
        b.control_points=a.control_points.clone();
        let ranges=[[[0.,1.],[1.,1.]];2];
        let fa=Fragment::new(&sa,&a,Endpoint::Parameter(2.5),Endpoint::Parameter(3.5)).unwrap();
        let fb=Fragment::new(&sb,&b,Endpoint::Parameter(16.),Endpoint::Parameter(12.)).unwrap();
        let partial=qualify(&world,[&fa,&fb],ranges,[None;2],100000000,0).unwrap().edge.unwrap();
        assert!(!partial.covers_complete_canonical_source(),"Original full affine carriers retain only the middle half");
        let restored=crate::source_shared_edge_restore::restore(partial.definition(),
            crate::source_shared_edge_restore::Limits {mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap().edge.unwrap();
        assert!(!restored.covers_complete_canonical_source());
        let fa=Fragment::new(&sa,&a,Endpoint::Parameter(2.),Endpoint::Parameter(4.)).unwrap();
        let fb=Fragment::new(&sb,&b,Endpoint::Parameter(18.),Endpoint::Parameter(10.)).unwrap();
        let full=qualify(&world,[&fa,&fb],ranges,[None;2],100000000,0).unwrap().edge.unwrap();
        assert!(full.covers_complete_canonical_source());
        let root=|s:&Surface,c:&Curve,x:f64,selector:[f64;2]| {
            let cutter=Curve::from_polyline(vec![vec![x,-1.],vec![x,1.]]).unwrap();
            let p=crate::source_contact_point::qualify(s,c,&cutter,[selector,[0.4,0.6]],10000).unwrap().point.unwrap();
            Endpoint::Crossing {point:p,role:Role::Boundary}
        };
        let fa=Fragment::new(&sa,&a,root(&sa,&a,0.4,[2.7,2.9]),root(&sa,&a,0.6,[3.1,3.3])).unwrap();
        let fb=Fragment::new(&sb,&b,root(&sb,&b,0.6,[14.7,14.9]),root(&sb,&b,0.4,[13.1,13.3])).unwrap();
        let owned=qualify(&world,[&fa,&fb],ranges,[None;2],100000000,0).unwrap().edge.unwrap();
        assert!(!owned.covers_complete_canonical_source());
        let restored=crate::source_shared_edge_restore::restore(owned.definition(),
            crate::source_shared_edge_restore::Limits {mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap().edge.unwrap();
        assert!(!restored.covers_complete_canonical_source());
    }
    #[test]
    fn reversed_affine_carriers_preserve_full_and_partial_retained_coverage() {
        let (sa,sb,mut a,mut b,world)=fixture();
        a.control_points.reverse();b.control_points=a.control_points.clone();
        let ranges=[[[1.,1.],[0.,1.]];2];
        for full in [false,true] {
            let ends=if full {[[2.,4.],[18.,10.]]} else {[[2.5,3.5],[16.,12.]]};
            let fa=Fragment::new(&sa,&a,Endpoint::Parameter(ends[0][0]),Endpoint::Parameter(ends[0][1])).unwrap();
            let fb=Fragment::new(&sb,&b,Endpoint::Parameter(ends[1][0]),Endpoint::Parameter(ends[1][1])).unwrap();
            let e=qualify(&world,[&fa,&fb],ranges,[None;2],100000000,0).unwrap().edge.unwrap();
            assert_eq!(e.covers_complete_canonical_source(),full);
            let sample=e.sample_interior(0.5,3).unwrap().sample.unwrap();
            let bounds=sample.world_bounds();
            assert!(bounds[0][0]<=0.5 && 0.5<=bounds[0][1]);
        }
    }
    #[test]
    fn different_geometric_coverage_has_exact_fixed_and_mixed_owned_ends() {
        let (sa, sb, a, b, world) = fixture();
        let fa =
            Fragment::new(&sa, &a, Endpoint::Parameter(2.5), Endpoint::Parameter(3.5)).unwrap();
        let fb =
            Fragment::new(&sb, &b, Endpoint::Parameter(18.), Endpoint::Parameter(10.)).unwrap();
        assert!(
            source_shared_edge::qualify(&world, [&fa, &fb], [false, false], 100_000_000)
                .unwrap()
                .edge
                .is_none()
        );
        let r = qualify(&world, [&fa, &fb], RANGES, [None, None], 100_000_000, 0).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        let e = r.edge.unwrap();
        assert_eq!(e.world(), &world);
        assert_eq!(e.uses()[1].curve(), &b);
        assert_eq!(e.ranges(), Some(RANGES));
        assert!(!e.covers_complete_canonical_source());
        assert_eq!(e.reversed(), [false, true]);
        for end in 0..2 {
            let report=e.sample_endpoint(end,3).unwrap();
            assert_eq!(report.spans,3);
            let endpoint=report.endpoint.expect("owned endpoint on differently covered source charts");
            let x=if end==0 {0.25}else{0.75};
            let canonical=endpoint.canonical_bounds();
            assert!(canonical[0]<=x && x<=canonical[1]);
            let world=endpoint.world_bounds();
            assert!(world[0][0]<=x && x<=world[0][1]);
            assert_eq!(endpoint.ends(),[end,1-end]);
            assert!(e.sample_endpoint(end,2).unwrap().endpoint.is_none());
        }
        assert!(e.sample_interior(0.5,3).unwrap().sample.is_some());
        for t in [0.,0.1,0.25,0.75,0.9,1.] {
            assert!(e.sample_interior(t,3).unwrap().sample.is_none());
        }
        let limits=|cells,spans|crate::source_shared_edge::BoundaryMeshLimits {
            max_deviation_mm:1e-6,cells,spans,max_depth:16,
        };
        let mesh=e.prepare_boundary_mesh(limits(100,100)).unwrap();
        assert!(mesh.pending.is_empty());
        assert_eq!(mesh.segments.len(),1);
        assert_eq!(mesh.vertices.len(),2);
        let restored=crate::source_shared_edge_restore::restore(e.definition(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000,
            }).unwrap().edge.expect("replay exact partial edge maps");
        let replay=restored.prepare_boundary_mesh(limits(100,100)).unwrap();
        assert!(replay.pending.is_empty());
        assert_eq!(replay.vertices.len(),mesh.vertices.len());
        assert_eq!(replay.segments.len(),mesh.segments.len());
        for (a,b) in replay.vertices.iter().zip(&mesh.vertices) {
            assert_eq!(a.canonical,b.canonical);
            assert_eq!(a.parameters,b.parameters);
            assert_eq!(a.uv,b.uv);
            assert_eq!(a.world,b.world);
        }
        let stopped=e.prepare_boundary_mesh(limits(0,100)).unwrap();
        assert_eq!(stopped.pending.len(),1);
        assert_eq!(stopped.pending[0].vertices,Some([0,1]));
        let chain=e.sample_interior_chain(&[0.1,0.375,0.5,0.625,0.9],6).unwrap();
        assert_eq!(chain.points.len(),5);
        assert_eq!(chain.pending,3);
        assert_eq!(chain.spans,6);
        let wrong = Fragment::new(
            &sb,
            &b,
            Endpoint::Parameter(18. - 1e-12),
            Endpoint::Parameter(10.),
        )
        .unwrap();
        assert!(
            qualify(&world, [&fa, &wrong], RANGES, [None, None], 100_000_000, 0)
                .unwrap()
                .edge
                .is_none()
        );
        let cut = Curve::from_polyline(vec![vec![0.75, -1.], vec![0.75, 1.]]).unwrap();
        let point =
            crate::source_contact_point::qualify(&sa, &a, &cut, [[3.4, 3.6], [0.4, 0.6]], 10000)
                .unwrap();
        assert!(point.point.is_some(), "{}", point.reason);
        let root = Fragment::new(
            &sa,
            &a,
            Endpoint::Parameter(2.5),
            Endpoint::Crossing {
                point: point.point.unwrap(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let plane = [[0.75, 0., 0.], [0.75, 1., 0.], [0.75, 0., 1.]];
        let r = qualify(
            &world,
            [&root, &fb],
            RANGES,
            [None, Some(plane)],
            100_000_000,
            1000,
        )
        .unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert!(r.driver_cells > 0);
        let mapped=r.edge.as_ref().unwrap();
        let endpoint=mapped.sample_endpoint(1,3).unwrap().endpoint.expect("root endpoint on partial canonical source");
        let bounds=endpoint.world_bounds();
        assert!(bounds[0][0]<=0.75 && 0.75<=bounds[0][1]);
        assert!(!mapped.covers_complete_canonical_source());
        assert!(qualify(
            &world,
            [&fb, &root],
            [RANGES[1], RANGES[0]],
            [Some(plane), None],
            100_000_000,
            1000
        )
        .unwrap()
        .edge
        .is_some());
        assert!(qualify(
            &world,
            [&root, &fb],
            RANGES,
            [None, Some(plane)],
            100_000_000,
            0
        )
        .unwrap()
        .edge
        .is_none());
        assert!(qualify(&world, [&fa, &fb], RANGES, [None, None], 1, 0)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn curved_partial_affine_edge_has_one_adaptive_partition_for_both_uses() {
        for rational in [false,true] {
        let (mut sa,mut sb,a,b,mut world)=fixture();
        for (surface,vertical) in [(&mut sa,false),(&mut sb,true)] {
            surface.degree_u=2;surface.knots_u=vec![0.,0.,0.,1.,1.,1.];
            surface.control_points=[(0.,0.),(0.5,0.5),(1.,0.)].into_iter().map(|(x,z)|
                [0.,1.].into_iter().map(|v|if vertical {vec![x,0.,z+v]} else {vec![x,v,z]}).collect()).collect();
            surface.weights=[1.,if rational {0.5}else{1.},1.].into_iter().map(|w|vec![w;2]).collect();
        }
        world.degree=2;world.knots=vec![-5.,-5.,-5.,3.,3.,3.];
        world.control_points=vec![vec![0.,0.,0.],vec![0.5,0.,0.5],vec![1.,0.,0.]];
        world.weights=vec![1.,if rational {0.5}else{1.},1.];
        let fa=Fragment::new(&sa,&a,Endpoint::Parameter(2.5),Endpoint::Parameter(3.5)).unwrap();
        let fb=Fragment::new(&sb,&b,Endpoint::Parameter(18.),Endpoint::Parameter(10.)).unwrap();
        let qualified=qualify(&world,[&fa,&fb],RANGES,[None;2],100000000,0).unwrap();
        let edge=qualified.edge.expect("exact curved partial affine uses");
        let limits=|cells,depth|crate::source_shared_edge::BoundaryMeshLimits {
            max_deviation_mm:0.001,cells,spans:10000,max_depth:depth,
        };
        let mesh=edge.prepare_boundary_mesh(limits(10000,16)).unwrap();
        assert!(mesh.pending.is_empty());assert!(mesh.segments.len()>1);
        for segment in &mesh.segments {
            assert!(segment.deviation_upper_mm<=0.001);
            let [a,b]=segment.vertices;
            assert!(mesh.vertices[a].canonical[1]<mesh.vertices[b].canonical[0]);
        }
        // Every partition vertex owns the two original UV enclosures, not two
        // independently rounded per-face approximations.
        for vertex in &mesh.vertices {
            for side in 0..2 {
                let d=edge.uses()[side].curve().domain();
                assert!(d[0]<=vertex.parameters[side][0] && vertex.parameters[side][1]<=d[1]);
            }
        }
        let stopped=edge.prepare_boundary_mesh(limits(10000,0)).unwrap();
        assert!(stopped.segments.is_empty());assert_eq!(stopped.pending.len(),1);
        assert_eq!(stopped.pending[0].reason,"boundary-mesh-depth-limit");
        assert_eq!(stopped.pending[0].vertices,Some([0,1]));
        assert!(!edge.covers_complete_canonical_source());
        let root_u=if rational {0.5}else{0.75};
        let root_t=if rational {3.}else{3.5};
        let root_fb=Fragment::new(&sb,&b,Endpoint::Parameter(if rational {14.}else{18.}),Endpoint::Parameter(10.)).unwrap();
        let cutter=Curve::from_polyline(vec![vec![root_u,-1.],vec![root_u,1.]]).unwrap();
        let contact=crate::source_contact_point::qualify(&sa,&a,&cutter,
            [[root_t-0.000001,root_t+0.000001],[0.49,0.51]],10000).unwrap().point.unwrap();
        let root=Fragment::new(&sa,&a,Endpoint::Parameter(2.5),
            Endpoint::Crossing {point:contact,role:Role::Boundary}).unwrap();
        let plane=[[root_u,0.,0.],[root_u,1.,0.],[root_u,0.,1.]];
        let admitted=qualify(&world,[&root,&root_fb],RANGES,[None,Some(plane)],100000000,10000).unwrap();
        let root_edge=admitted.edge.expect("curved partial edge with exact common root end");
        let root_mesh=root_edge.prepare_boundary_mesh(limits(10000,16)).unwrap();
        assert!(root_mesh.pending.is_empty());assert!(root_mesh.segments.len()>1);
        assert!(root_mesh.vertices[1].canonical[0]<=root_u && root_u<=root_mesh.vertices[1].canonical[1]);
        assert!(root_mesh.segments.iter().all(|s|s.deviation_upper_mm<=0.001));
        }
    }
    #[test]
    fn independently_partitioned_uv_uses_replay_one_partial_canonical_edge() {
        let (sa,sb,mut a,mut b,mut world)=fixture();
        a.knots=vec![2.,2.,2.5,3.5,4.,4.];
        a.control_points=vec![vec![0.,0.],vec![0.25,0.],vec![0.75,0.],vec![1.,0.]];
        a.weights=vec![1.;4];
        b.knots=vec![10.,10.,14.,18.,18.];
        b.control_points=vec![vec![0.25,0.],vec![0.5,0.],vec![0.75,0.]];
        b.weights=vec![1.;3];
        world.knots=vec![-5.,-5.,-1.,3.,3.];
        world.control_points=vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,0.,0.]];
        world.weights=vec![1.;3];
        let fa=Fragment::new(&sa,&a,Endpoint::Parameter(2.5),Endpoint::Parameter(3.5)).unwrap();
        let fb=Fragment::new(&sb,&b,Endpoint::Parameter(18.),Endpoint::Parameter(10.)).unwrap();
        let report=qualify(&world,[&fa,&fb],RANGES,[None;2],100000000,0).unwrap();
        let edge=report.edge.expect(report.reason);
        assert!(!edge.covers_complete_canonical_source());
        assert_eq!(edge.uses()[0].curve(),&a);
        assert_eq!(edge.uses()[1].curve(),&b);
        let restored=crate::source_shared_edge_restore::restore(edge.definition(),
            crate::source_shared_edge_restore::Limits{mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap();
        assert_eq!(restored.edge.expect(restored.reason).definition(),edge.definition());
        let bad=Fragment::new(&sb,&b,Endpoint::Parameter(18.-1e-12),Endpoint::Parameter(10.)).unwrap();
        assert!(qualify(&world,[&fa,&bad],RANGES,[None;2],100000000,0).unwrap().edge.is_none());
        let root=|surface:&Surface,curve:&Curve,x:f64,t:f64| {
            let cutter=Curve::from_polyline(vec![vec![x,-1.],vec![x,1.]]).unwrap();
            let report=crate::source_contact_point::qualify(surface,curve,&cutter,
                [[t-0.05,t+0.05],[0.4,0.6]],10000).unwrap();
            Endpoint::Crossing{point:report.point.expect(report.reason),role:Role::Boundary}
        };
        let ra=Fragment::new(&sa,&a,root(&sa,&a,0.375,2.75),root(&sa,&a,0.625,3.25)).unwrap();
        let rb=Fragment::new(&sb,&b,root(&sb,&b,0.625,16.),root(&sb,&b,0.375,12.)).unwrap();
        let candidates=[[Some([2.75,0.5]),Some([3.25,0.5])],[Some([16.,0.5]),Some([12.,0.5])]];
        let report=qualify_with_candidates(&world,[&ra,&rb],RANGES,[None;2],candidates,100000000,0).unwrap();
        let rooted=report.edge.expect(report.reason);
        assert_eq!(report.root_checks,4);
        assert!(rooted.uses().iter().all(|fragment|fragment.endpoints().iter().all(|end|
            matches!(end,Endpoint::Crossing{..}))));
        let restored=crate::source_shared_edge_restore::restore(rooted.definition(),
            crate::source_shared_edge_restore::Limits{mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap();
        assert_eq!(restored.edge.expect(restored.reason).definition(),rooted.definition());
        let mut displaced=candidates;displaced[1][0]=Some([16.+1e-12,0.5]);
        assert!(qualify_with_candidates(&world,[&ra,&rb],RANGES,[None;2],displaced,100000000,0).unwrap().edge.is_none());
    }
    #[test]
    fn independent_irrational_roots_on_multispan_uv_uses_keep_owned_plane_relation() {
        let (sa,sb,mut a,mut b,mut world)=fixture();
        // x(q)=(q+q*q)/2 has strictly positive derivative. Its roots for
        // x=1/4 and x=1/2 are irrational; no binary64 parameter is supplied.
        world.degree=2;world.knots=vec![-5.,-5.,-5.,3.,3.,3.];
        world.control_points=vec![vec![0.,0.,0.],vec![0.25,0.,0.],vec![1.,0.,0.]];
        world.weights=vec![1.;3];
        a.degree=2;a.knots=vec![2.,2.,2.,3.,4.,4.,4.];
        a.control_points=[0.,0.125,0.625,1.].into_iter().map(|x|vec![x,0.]).collect();
        a.weights=vec![1.;4];
        b.degree=2;b.knots=vec![10.,10.,10.,12.,14.,18.,18.,18.];
        b.control_points=[0.,0.0625,0.25,0.625,1.].into_iter().map(|x|vec![x,0.]).collect();
        b.weights=vec![1.;5];
        let root=|surface:&Surface,curve:&Curve,x:f64,selector:[f64;2]| {
            let cutter=Curve::from_polyline(vec![vec![x,-1.],vec![x,1.]]).unwrap();
            let report=crate::source_contact_point::qualify(surface,curve,&cutter,
                [selector,[0.4,0.6]],10000).unwrap();
            Endpoint::Crossing{point:report.point.expect(report.reason),role:Role::Boundary}
        };
        let fa=Fragment::new(&sa,&a,root(&sa,&a,0.25,[2.73,2.74]),root(&sa,&a,0.5,[3.23,3.24])).unwrap();
        let fb=Fragment::new(&sb,&b,root(&sb,&b,0.5,[14.94,14.95]),root(&sb,&b,0.25,[12.92,12.94])).unwrap();
        let plane=|x|[[x,0.,0.],[x,1.,0.],[x,0.,1.]];
        let ranges=[[[0.,1.],[1.,1.]];2];
        let planes=[Some(plane(0.25)),Some(plane(0.5))];
        let mut split=world.clone();split.knots=vec![-5.,-5.,-5.,-1.,3.,3.,3.];
        split.control_points=[0.,0.125,0.625,1.].into_iter().map(|x|vec![x,0.,0.]).collect();
        split.weights=vec![1.;4];
        for world in [world,split] {
        let report=qualify(&world,[&fa,&fb],ranges,planes,100000000,10000).unwrap();
        let edge=report.edge.expect(report.reason);
        assert!(report.driver_cells>0);
        assert_eq!(report.root_checks,2);
        assert!(!edge.covers_complete_canonical_source());
        assert!(edge.uses().iter().all(|f|f.endpoints().iter().all(|e|matches!(e,Endpoint::Crossing{..}))));
        let restored=crate::source_shared_edge_restore::restore(edge.definition(),
            crate::source_shared_edge_restore::Limits{mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap();
        assert_eq!(restored.edge.expect(restored.reason).definition(),edge.definition());
        assert!(qualify(&world,[&fa,&fb],ranges,planes,100000000,0).unwrap().edge.is_none());
        assert!(qualify(&world,[&fa,&fb],ranges,[None;2],100000000,10000).unwrap().edge.is_none());
        assert!(qualify(&world,[&fa,&fb],ranges,[planes[0],Some(plane(0.5+1e-12))],100000000,10000).unwrap().edge.is_none());
        }
    }
    #[test]
    fn local_plane_uniqueness_matches_one_branch_and_refuses_opposite_branch_roots() {
        for oblique in [false,true] {
        let (mut sa,mut sb,mut a,mut b,mut world)=fixture();
        // x(q)=(2q-1)^2 crosses each chosen plane twice globally.
        world.degree=2;world.knots=vec![-5.,-5.,-5.,3.,3.,3.];
        world.control_points=vec![vec![1.,0.,0.],vec![-1.,0.,0.],vec![1.,0.,0.]];world.weights=vec![1.;3];
        a.degree=2;a.knots=vec![2.,2.,2.,3.,4.,4.,4.];
        a.control_points=[1.,0.,0.,1.].into_iter().map(|x|vec![x,0.]).collect();a.weights=vec![1.;4];
        b.degree=2;b.knots=vec![10.,10.,10.,12.,14.,18.,18.,18.];
        b.control_points=[1.,0.5,0.,0.,1.].into_iter().map(|x|vec![x,0.]).collect();b.weights=vec![1.;5];
        if oblique {
            // Apply the same exact affine shear to both original surfaces
            // and their canonical curve; the UV root owners stay unchanged.
            let shear=|p:&mut Vec<f64>| {let (x,y)=(p[0],p[1]);p[0]=x+y;p[1]=x+2.*y;};
            for p in &mut world.control_points {shear(p);}
            for surface in [&mut sa,&mut sb] {
                for row in &mut surface.control_points {for p in row {shear(p);}}
            }
        }
        let root=|surface:&Surface,curve:&Curve,x:f64,selector:[f64;2]| {
            let cutter=Curve::from_polyline(vec![vec![x,-1.],vec![x,1.]]).unwrap();
            let report=crate::source_contact_point::qualify(surface,curve,&cutter,[selector,[0.4,0.6]],10000).unwrap();
            Endpoint::Crossing{point:report.point.expect(report.reason),role:Role::Boundary}
        };
        let fa=Fragment::new(&sa,&a,root(&sa,&a,0.5,[2.29,2.30]),root(&sa,&a,0.125,[2.64,2.65])).unwrap();
        let fb=Fragment::new(&sb,&b,root(&sb,&b,0.125,[12.58,12.59]),root(&sb,&b,0.5,[11.17,11.18])).unwrap();
        let plane=|x|if oblique {[[x,x,0.],[x+1.,x+2.,0.],[x,x,1.]]} else {[[x,0.,0.],[x,1.,0.],[x,0.,1.]]};
        let planes=[Some(plane(0.5)),Some(plane(0.125))];let ranges=[[[0.,1.],[1.,1.]];2];
        assert!(!nurbs_core::curve_axis_driver::certify(&world,0,128).unwrap().monotonic_proven);
        let report=qualify(&world,[&fa,&fb],ranges,planes,100000000,10000).unwrap();
        let edge=report.edge.expect(report.reason);assert_eq!(report.root_checks,2);assert!(report.driver_cells>0);
        let replay=crate::source_shared_edge_restore::restore(edge.definition(),
            crate::source_shared_edge_restore::Limits{mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap();
        assert_eq!(replay.edge.expect(replay.reason).definition(),edge.definition());
        // A fixed original parameter must match the owned root on the same
        // branch, even when that plane also meets another canonical branch.
        let mixed_a=Fragment::new(&sa,&a,root(&sa,&a,0.25,[2.49,2.51]),
            root(&sa,&a,0.125,[2.64,2.65])).unwrap();
        let mixed_b=Fragment::new(&sb,&b,root(&sb,&b,0.125,[12.58,12.59]),
            Endpoint::Parameter(12.)).unwrap();
        let mixed_planes=[Some(plane(0.25)),Some(plane(0.125))];
        let mixed=qualify(&world,[&mixed_a,&mixed_b],ranges,mixed_planes,100000000,10000).unwrap();
        let mixed_edge=mixed.edge.expect(mixed.reason);
        let restored=crate::source_shared_edge_restore::restore(mixed_edge.definition(),
            crate::source_shared_edge_restore::Limits{mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap();
        assert_eq!(restored.edge.expect(restored.reason).definition(),mixed_edge.definition());
        let mixed_wrong=Fragment::new(&sb,&b,root(&sb,&b,0.125,[12.58,12.59]),
            Endpoint::Parameter(16.)).unwrap();
        assert!(qualify(&world,[&mixed_a,&mixed_wrong],ranges,mixed_planes,100000000,256).unwrap().edge.is_none());
        // Same physical plane/point, but the other canonical branch must not
        // be merged into the left branch by an overlap or midpoint heuristic.
        let wrong=Fragment::new(&sb,&b,root(&sb,&b,0.125,[15.41,15.42]),root(&sb,&b,0.5,[11.17,11.18])).unwrap();
        assert!(qualify(&world,[&fa,&wrong],ranges,planes,100000000,256).unwrap().edge.is_none());
        assert!(qualify(&world,[&fa,&fb],ranges,planes,100000000,0).unwrap().edge.is_none());
        assert!(qualify(&world,[&fa,&fb],ranges,[None;2],100000000,10000).unwrap().edge.is_none());
        }
    }
    #[test]
    fn linear_roots_on_different_source_coverage_keep_exact_canonical_parameters() {
        let (sa, sb, a, b, world) = fixture();
        let point = |s: &Surface, c: &Curve, x: f64, selector: [f64; 2]| {
            let cut = Curve::from_polyline(vec![vec![x, -1.], vec![x, 1.]]).unwrap();
            let r = crate::source_contact_point::qualify(s, c, &cut, [selector, [0.4, 0.6]], 10000)
                .unwrap();
            assert!(r.point.is_some(), "{}", r.reason);
            Endpoint::Crossing {
                point: r.point.unwrap(),
                role: Role::Boundary,
            }
        };
        let fa = Fragment::new(
            &sa,
            &a,
            point(&sa, &a, 0.4, [2.7, 2.9]),
            point(&sa, &a, 0.6, [3.1, 3.3]),
        )
        .unwrap();
        let fb = Fragment::new(
            &sb,
            &b,
            point(&sb, &b, 0.6, [15.5, 15.7]),
            point(&sb, &b, 0.4, [12.3, 12.5]),
        )
        .unwrap();
        let mut split_world=world.clone();
        split_world.knots=vec![-5.,-5.,-1.,3.,3.];
        split_world.control_points.insert(1,vec![0.5,0.,0.]);
        split_world.weights.insert(1,1.);
        for world in [world,split_world] {
        let r = qualify(&world, [&fa, &fb], RANGES, [None, None], 100_000_000, 0).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.driver_cells, 0);
        let e = r.edge.unwrap();
        let replay=crate::source_shared_edge_restore::restore(e.definition(),
            crate::source_shared_edge_restore::Limits{mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap();
        assert_eq!(replay.edge.expect(replay.reason).definition(),e.definition());
        assert!(e.uses().iter().all(|f| f
            .endpoints()
            .iter()
            .all(|p| matches!(p, Endpoint::Crossing { .. }))));
        let wrong = Fragment::new(
            &sb,
            &b,
            point(&sb, &b, 0.6 + 1e-12, [15.5, 15.7]),
            point(&sb, &b, 0.4, [12.3, 12.5]),
        )
        .unwrap();
        assert!(
            qualify(&world, [&fa, &wrong], RANGES, [None, None], 100_000_000, 0)
                .unwrap()
                .edge
                .is_none()
        );
        }
    }
    #[test]
    fn nonlinear_root_fixed_end_has_exact_candidate_without_plane_hints() {
        let (sa, _, _, _, _) = fixture();
        let uv = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let world = Curve {
            control_points: uv
                .control_points
                .iter()
                .map(|p| vec![p[0], p[1], 0.])
                .collect(),
            ..uv.clone()
        };
        let sb = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: uv.knots.clone(),
            control_points: (0..2)
                .map(|z| {
                    uv.control_points
                        .iter()
                        .map(|p| vec![p[0], p[1], z as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let side = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        let cut = Curve::from_polyline(vec![vec![0., 0.859375], vec![1., 0.859375]]).unwrap();
        let point = crate::source_contact_point::qualify(
            &sa,
            &uv,
            &cut,
            [[0.6, 0.65], [0.58, 0.63]],
            10000,
        )
        .unwrap()
        .point
        .unwrap();
        let definition = point.definition();
        let fa = Fragment::new(
            &sa,
            &uv,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: point.clone(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let fb = Fragment::new(
            &sb,
            &side,
            Endpoint::Parameter(0.625),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let ranges = [[[0., 1.], [1., 1.]]; 2];
        assert!(
            qualify(&world, [&fa, &fb], ranges, [None; 2], 100_000_000, 0)
                .unwrap()
                .edge
                .is_none()
        );
        let candidates = [[None, Some([0.625, 0.609375])], [None, None]];
        let report = qualify_with_candidates(
            &world,
            [&fa, &fb],
            ranges,
            [None; 2],
            candidates,
            100_000_000,
            0,
        )
        .unwrap();
        assert!(report.edge.is_some(), "{}", report.reason);
        let edge = report.edge.unwrap();
        crate::source_edge_restriction::assert_replay(&edge);
        let restriction = crate::source_edge_restriction::Restriction::from_edge(&edge);
        let b = restriction.parameter_bounds().unwrap()[1];
        assert!(b[0] <= 0.625 && 0.625 <= b[1]);
        let b = restriction.endpoint_boxes(100).unwrap()[1];
        for (axis, expected) in [0.609375, 0.859375, 0.].into_iter().enumerate() {
            assert!(b[axis][0] <= expected && expected <= b[axis][1]);
        }
        let restore = |value| crate::source_shared_edge_restore::restore(value,
            crate::source_shared_edge_restore::Limits {mapping_cells_per_use:10000,exact_work:100_000_000,driver_cells:10000}).unwrap();
        let restored=restore(edge.definition()).edge.unwrap();
        assert_eq!(restored.definition(),edge.definition());
        assert_eq!(restored.ranges(),edge.ranges());
        assert_eq!(restored.reversed(),edge.reversed());
        let mut displaced=edge.definition();
        displaced["recipe"]["candidates"][0][1][0]=value_codec::json!(0.625+1e-12);
        displaced["certificateQualified"]=value_codec::json!(true);
        assert!(restore(displaced).edge.is_none());

        match &edge.uses()[0].endpoints()[1] {
            Endpoint::Crossing { point, .. } => assert_eq!(point.definition(), definition),
            _ => panic!("root expression was replaced"),
        }
        assert!(qualify_with_candidates(
            &world,
            [&fa, &fb],
            ranges,
            [None; 2],
            [[None, Some([0.625 + 1e-12, 0.609375])], [None, None]],
            100_000_000,
            0
        )
        .unwrap()
        .edge
        .is_none());
        assert!(crate::source_root_parameter::verify(
            &point,
            [0.6, 0.609375],
            cad_predicates::MAX_WORK
        )
        .unwrap()
        .parameters
        .is_none());
        assert!(
            crate::source_root_parameter::verify(&point, [0.625, 0.609375], 1)
                .unwrap()
                .parameters
                .is_none()
        );
    }
}
