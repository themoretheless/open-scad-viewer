//! Non-Mobius rational coordinates retain independent global/rank/G2 owners.
use super::*;
use nurbs_core::curve::Curve;
fn profile(z:f64,rank_loss:bool)->Vec<Curve> {
    let weights=vec![1.,2.,2.,2.,2.,1.];
    let c=Curve{degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),control_points:(0..6).map(|i|vec![if i%2==0 {-10./weights[i]}else{10./weights[i]},(if rank_loss {let k=i as i64;(2*(5-2*k)*if i%2==0 {1}else{-1}) as f64}else{2.*(i*i)as f64-9.*i as f64+7.5})/weights[i],z]).collect(),weights:weights.clone(),periodic:false};
    let line=|a:Vec<f64>,b:Vec<f64>|Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![a,b],weights:vec![1.;2],periodic:false};
    vec![c.clone(),line(c.control_points[5].clone(),vec![0.,10.,z]),line(vec![0.,10.,z],c.control_points[0].clone())]
}
fn rational_body_with(rank_loss:bool)->Model {
    let sections=(0..3).map(|i|vec![profile(i as f64,rank_loss)]).collect::<Vec<_>>();
    let mut model=crate::rational_section_loft(&sections).unwrap();
    for f in model.faces.len()-2..model.faces.len() {
        let s=&mut model.0.faces[f].surface;let a=s.control_points[0][0].clone();let b=s.control_points[1][1].clone();
        s.knots_u=vec![a[0],a[0],b[0],b[0]];s.knots_v=vec![a[1],a[1],b[1],b[1]];
        let wire=model.faces[f].outer;
        for c in &mut model.0.loops[wire].coedges {
            let mut p=model.0.edges[c.edge].curve.clone();for point in &mut p.control_points {point.truncate(2);}
            if c.reversed {p=p.reverse().unwrap();}c.pcurve=p;
        }
    }
    model.validate().unwrap();model
}
#[test]
fn non_mobius_station_requires_independent_boundary_volume_and_g2() {
    let model=rational_body_with(false);let before=model.clone();
    let full=inspect(&model,&model,1e-8,0.,limits()).unwrap();
    assert!(full.certified,"reason={:?} volume={:?}",full.reason,full.reference_volume.as_ref().map(|v| &v.boundary.intersections.pairs));
    assert!(full.continuous_bound && full.error_upper==Some(0.));
    let caps=[6,7];
    let g=crate::sweep_smoothness::inspect_axis(&model,&caps,crate::sweep_smoothness::Axis::Station,2_000_000).unwrap();
    assert!(g.extraction_complete && g.g1_certified && g.g2_certified);
    assert_eq!(g.edge_ids.len(),3);
    let short_g=crate::sweep_smoothness::inspect_axis(&model,&caps,crate::sweep_smoothness::Axis::Station,1).unwrap();
    assert!(!short_g.g2_certified);
    let mut short=limits();short.reference_volume.boundary.contacts.cells=1;
    let refused=inspect(&model,&model,1e-8,0.,short).unwrap();
    assert!(refused.continuous_bound && !refused.certified);
    assert_eq!(model,before);
}
#[test]
fn non_mobius_support_owns_exact_edges_vertices_and_one_shared_budget() {
    use crate::shared_boundary::opposite_rows::certify_restricted as certify;
    let model=rational_body_with(false);let before=model.clone();
    for pair in [[0,1],[0,4]] {
        let full=certify(&model,pair,1000);
        let proof=full.certificate.unwrap_or_else(||panic!("pair={pair:?} work={}",full.cells));
        if pair==[0,1] {assert_eq!(proof.edges.len(),1);assert!(proof.vertex.is_none());}
        else {assert!(proof.edges.is_empty());assert!(proof.vertex.is_some());}
        let n=full.cells;
        assert!(certify(&model,pair,n).certificate.is_some());
        let short=certify(&model,pair,n-1);assert!(short.certificate.is_none());assert_eq!(short.cells,n-1);
        assert!(certify(&model,pair,0).certificate.is_none());
    }
    let mut unowned=model.clone();
    // Coordinate equality alone is not vertex ownership.
    let wire=unowned.faces[4].outer;
    for c in &mut unowned.0.loops[wire].coedges {
        let mut e=unowned.0.edges[c.edge].clone();
        for v in &mut e.vertices {let p=unowned.0.vertices[*v].clone();*v=unowned.0.vertices.len();unowned.0.vertices.push(p);}
        c.edge=unowned.0.edges.len();unowned.0.edges.push(e);
    }
    assert!(certify(&unowned,[0,4],1000).certificate.is_none());
    let mut crossed=model.clone();crossed.faces[0].surface.control_points[2][1][1]+=100.;
    assert!(certify(&crossed,[0,1],1000).certificate.is_none());
    let mut invalid=model.clone();invalid.faces[0].surface.weights[2][1]=0.;
    assert!(certify(&invalid,[0,1],1000).certificate.is_none());
    assert_eq!(model,before);
}

#[test]
fn non_mobius_injectivity_never_transfers_immersion_across_a_critical_point() {
    // C(q)=10(q^5,q^4)/W(q), q=2u-1. Both tangent components
    // vanish exactly at u=1/2; injection alone cannot prove rank.
    let model=rational_body_with(true);
    assert!(nurbs_core::surface_injectivity::certify_chart(&model.faces[0].surface,1000).unwrap().proven);
    let mut budget=limits();budget.regularity_cells=100;
    let report=inspect(&model,&model,1e-8,0.,budget).unwrap();
    assert!(report.continuous_bound && report.error_upper==Some(0.));
    assert!(!report.certified);
    assert_eq!(report.reason,Some("whole-boundary-regularity-unproved"));
}
fn station_clock(model:&mut Model) {
    let end=model.faces.len()-2;
    for face in &mut model.0.faces[..end] {
        let s=&mut face.surface;s.degree_v=2;s.knots_v=vec![0.,0.,0.,1.,1.,1.];
        for (p,w)in s.control_points.iter_mut().zip(&mut s.weights) {
            let mid=(0..3).map(|k|(p[0][k]+p[1][k])/2.).collect();
            *p=vec![p[0].clone(),mid,p[1].clone()];*w=vec![w[0],2.*w[0],w[1]];
        }
    }
    for e in &mut model.0.edges {
        let c=&mut e.curve;
        if c.degree==1 && c.control_points[0][..2]==c.control_points[1][..2] && c.control_points[0][2]!=c.control_points[1][2] {
            let p=&c.control_points;let mid=(0..3).map(|k|(p[0][k]+p[1][k])/2.).collect();
            c.control_points=vec![p[0].clone(),mid,p[1].clone()];c.degree=2;c.knots=vec![0.,0.,0.,1.,1.,1.];
            c.weights=vec![c.weights[0],2.*c.weights[0],c.weights[1]];
        }
    }
    model.validate().unwrap();
}

#[test]
fn non_mobius_both_axes_require_actual_g2_despite_transverse_c2_jump() {
    let mut model=rational_body_with(false);station_clock(&mut model);let before=model.clone();
    let full=inspect(&model,&model,1e-8,0.,limits()).unwrap();
    assert!(full.certified,"{:?}; {:?}",full.reason,full.reference_volume.as_ref().map(|v|&v.boundary.intersections.pairs));
    assert!(full.continuous_bound && full.error_upper==Some(0.));
    let g=crate::sweep_smoothness::inspect_axis(&model,&[6,7],crate::sweep_smoothness::Axis::Station,2_000_000).unwrap();
    assert!(g.extraction_complete && g.g1_certified && g.g2_certified,"g1={} g2={} complete={}",g.g1_certified,g.g2_certified,g.extraction_complete);assert_eq!(g.edge_ids.len(),3);
    // z(v)=(2v-v^2)/(1+2v-2v^2), z'(0)=z'(1)=2,
    // z''(0)=-10, z''(1)=10. C2 fails; G2 must prove the
    // tangential transverse reparameterization, never copy a C2 flag.
    let left=model.faces[0].surface.evaluate(0.5,1.).unwrap();
    let right=model.faces[3].surface.evaluate(0.5,0.).unwrap();
    assert_eq!(left.first_derivatives().unwrap().1,right.first_derivatives().unwrap().1);
    assert_ne!(left.second_derivatives().unwrap().2,right.second_derivatives().unwrap().2);
    assert_eq!(model,before);
}

#[test]
fn non_mobius_nonbinary_factor_products_keep_exact_rational_ownership() {
    let mut model=rational_body_with(false);station_clock(&mut model);
    let mut s=model.faces[0].surface.clone();
    for row in &mut s.weights {
        let base=row[0];*row=vec![base*0.1,base*0.3,base*0.1];
    }
    // 0.6*0.1 and 0.2*0.3 are not representable products. Their
    // original binary64 rationals are exactly equal by power-two scaling.
    // The factor certificate must fall back to exact rationals, not compare
    // rounded products or require exactly representable arithmetic.
    let before=s.clone();let r=nurbs_core::surface_injectivity::certify_chart(&s,1000).unwrap();
    assert!(r.proven,"{r:?}");assert_eq!(r.reason,"global-exact-rational-separable-monotonicity");
    assert!(r.contraction_upper.is_none() && r.polar_p_matrix.is_none());
    let mut drift=s.clone();drift.weights[2][1]=drift.weights[2][1].next_up();
    assert!(!nurbs_core::surface_injectivity::certify_chart(&drift,1000).unwrap().proven);
    assert_eq!(s,before);
}

fn coupled_wall_body(coupled_coordinates: bool, monotone_rails: bool) -> Model {
    // Interior weights vary across both axes. The optional height offset
    // couples the station coordinate to the profile parameter as well.
    let corners = [[0., 0.], [4., 0.], [4., 4.], [0., 4.]];
    let sections = [0., 7., 14.].map(|z| {
        vec![
            (0..4)
                .map(|i| {
                    let [x, y] = corners[i];
                    let [xx, yy] = corners[(i + 1) % 4];
                    crate::line(vec![x, y, z], vec![xx, yy, z])
                })
                .collect::<Vec<_>>(),
        ]
    });
    let mut model = crate::rational_section_loft(&sections).unwrap();
    let middle_weight = |xy: &[f64]| {
        if xy[0] == xy[1] {
            1.
        } else if monotone_rails {
            32.
        } else if coupled_coordinates {
            2.
        } else {
            16.
        }
    };
    let station_poles = |a: &[f64], b: &[f64], w: f64| {
        // The first and last three coefficients are unchanged, so the
        // coupled interior weights do not alter the seam jets through order two.
        (0..=7)
            .map(|i| {
                let distance = if monotone_rails {
                    i as f64
                } else {
                    match i {
                        3 => 3. / w,
                        4 => 7. - 3. / w,
                        _ => i as f64,
                    }
                };
                let offset = if coupled_coordinates && !monotone_rails && w > 1. && i == 3 {
                    0.125
                } else {
                    0.
                };
                vec![a[0], a[1], a[2] + distance * ((b[2] - a[2]) / 7.) + offset]
            })
            .collect()
    };
    let wall_count = model.faces.len() - 2;
    let knots = [vec![0.; 8], vec![1.; 8]].concat();
    let weights_for = |w| vec![1., 1., 1., w, w, 1., 1., 1.];
    for face in &mut model.0.faces[..wall_count] {
        let surface = &mut face.surface;
        surface.degree_v = 7;
        surface.knots_v = knots.clone();
        for (row, weights) in surface.control_points.iter_mut().zip(&mut surface.weights) {
            let w = middle_weight(&row[0]);
            *row = station_poles(&row[0], &row[1], w);
            *weights = weights_for(w);
        }
    }
    for edge in &mut model.0.edges {
        let curve = &mut edge.curve;
        let [a, b] = curve.control_points.as_slice() else {
            continue;
        };
        if a[..2] == b[..2] && a[2] != b[2] {
            let w = middle_weight(a);
            curve.control_points = station_poles(a, b, w);
            curve.degree = 7;
            curve.knots = knots.clone();
            curve.weights = weights_for(w);
        }
    }
    model.validate().unwrap();
    model
}

#[test]
fn coupled_rational_walls_have_continuous_global_and_station_certificates() {
    check_coupled_wall_certificates(false, 1., false);
}

#[test]
fn coupled_coordinates_without_affine_recovery_keep_joint_certificates() {
    check_coupled_wall_certificates(true, 1., false);
}

#[test]
fn coupled_coordinates_without_column_dominance_keep_joint_certificates() {
    check_coupled_wall_certificates(true, 16., false);
}

#[test]
fn coupled_rails_keep_joint_certificates_after_independent_column_refusal() {
    check_coupled_wall_certificates(true, 1., true);
}

fn check_coupled_wall_certificates(
    coupled_coordinates: bool,
    profile_scale: f64,
    monotone_rails: bool,
) {
    let model = coupled_wall_body(coupled_coordinates, monotone_rails);
    let model = if profile_scale == 1. {
        model
    } else {
        crate::transform::affine(
            &model,
            [
                [profile_scale, 0., 0., 0.],
                [0., profile_scale, 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    };
    let wall_count = model.faces.len() - 2;
    let surface = &model.faces[0].surface;
    assert_ne!(surface.weights[0][3], surface.weights[1][3]);
    let chart = nurbs_core::surface_injectivity::certify_chart(surface, 1000).unwrap();
    assert!(chart.proven, "{chart:?}");
    let expected = if coupled_coordinates {
        let (du, dv) = surface
            .evaluate(0.5, 0.25)
            .unwrap()
            .first_derivatives()
            .unwrap();
        // On the first wall, y is constant. The tangent directions at
        // the two natural edges exclude any affine combination of x
        // and z that could recover one parameter throughout the chart.
        let (_, left_dv) = surface
            .evaluate(0., 0.25)
            .unwrap()
            .first_derivatives()
            .unwrap();
        let (bottom_du, _) = surface
            .evaluate(0.5, 0.)
            .unwrap()
            .first_derivatives()
            .unwrap();
        assert_eq!(left_dv[0], 0.);
        assert_eq!(bottom_du[2], 0.);
        assert_ne!(left_dv[2], 0.);
        assert_ne!(bottom_du[0], 0.);
        assert_ne!(du[2], 0.);
        assert_ne!(dv[0], 0.);
        let mut folded = surface.clone();
        // Along u=0 the wall stays on one vertical line. This control
        // displacement makes its height exceed both endpoints at v=1/2,
        // forcing a repeated world point on that natural edge.
        folded.control_points[0][3][2] += 1000.;
        assert!(
            folded.evaluate(0., 0.5).unwrap().point[2] > folded.evaluate(0., 1.).unwrap().point[2]
        );
        assert!(
            !nurbs_core::surface_injectivity::certify_chart(&folded, 1000)
                .unwrap()
                .proven
        );
        if monotone_rails {
            // The independent column route runs first and must refuse.
            "global-rational-monotone-rails"
        } else if profile_scale > 1. {
            assert!(dv[0].abs() > dv[2].abs(), "column V: {dv:?}");
            "global-rational-bernstein-column-determinant"
        } else {
            "global-rational-bernstein-column-cones"
        }
    } else {
        let mut drift = surface.clone();
        drift.control_points[0][3][2] = drift.control_points[0][3][2].next_up();
        let changed = nurbs_core::surface_injectivity::certify_chart(&drift, 1000).unwrap();
        assert_ne!(changed.reason, chart.reason);
        "global-exact-rational-triangular"
    };
    assert_eq!(chart.reason, expected);
    assert!(chart.contraction_upper.is_none());
    assert!(
        !nurbs_core::surface_injectivity::certify_chart(surface, chart.spans - 1)
            .unwrap()
            .proven
    );

    let mut retained = model.clone();
    for face in &mut retained.0.faces[..wall_count] {
        for weight in face.surface.weights.iter_mut().flatten() {
            *weight *= 2.;
        }
    }
    let full = inspect(&model, &retained, 1e-8, 0., limits()).unwrap();
    assert!(full.continuous_bound && full.error_upper == Some(0.));
    assert!(
        full.certified,
        "{:?}; geometry {:?}",
        full.reason,
        full.reference_volume.as_ref().map(|volume| (
            volume.boundary.agreement.all_equal,
            volume.boundary.intersections.faces.all_faces_injective,
            volume.boundary.intersections.pairs.all_pairs_classified,
            &volume.boundary.intersections.faces,
            volume
                .boundary
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|pair| pair.reason == "pair-unresolved")
                .map(|pair| pair.faces)
                .collect::<Vec<_>>(),
        ))
    );
    let mut short = limits();
    short.reference_volume.boundary.contacts.cells = 1;
    let refused = inspect(&model, &retained, 1e-8, 0., short).unwrap();
    assert!(refused.continuous_bound && !refused.certified);
    let caps = [wall_count, wall_count + 1];
    let station = crate::sweep_smoothness::inspect_axis(
        &retained,
        &caps,
        crate::sweep_smoothness::Axis::Station,
        2_000_000,
    )
    .unwrap();
    assert!(station.extraction_complete && station.g1_certified && station.g2_certified);
    assert_eq!(station.edge_ids.len(), 4);
}

#[test]
fn rectangular_rational_wall_outside_the_rails_class_keeps_joint_certificates() {
    check_polygonal_rational_wall_certificates(false, false, 0);
}

#[test]
fn oblique_quadrilateral_rational_walls_keep_joint_certificates() {
    check_polygonal_rational_wall_certificates(true, false, 0);
}

#[test]
fn curved_nonconvex_wall_projection_keeps_joint_certificates() {
    check_polygonal_rational_wall_certificates(false, true, 0);
}

#[test]
fn multispan_curved_nonconvex_wall_projection_keeps_joint_certificates() {
    check_polygonal_rational_wall_certificates(false, true, 1);
}

#[test]
fn full_multiplicity_nonconvex_wall_projection_keeps_joint_certificates() {
    check_polygonal_rational_wall_certificates(false, true, 4);
}

fn check_polygonal_rational_wall_certificates(oblique: bool, curved_seam: bool, insertions: usize) {
    let multispan = insertions > 0;
    let corners = [[0., 0.], [4., 0.], [4., 4.], [0., 4.]];
    let sections = [0., 4., 8.].map(|z| {
        vec![(0..4)
            .map(|i| {
                let [x, y] = corners[i];
                let [xx, yy] = corners[(i + 1) % 4];
                crate::line(vec![x, y, z], vec![xx, yy, z])
            })
            .collect::<Vec<_>>()]
    });
    let mut reference = crate::rational_section_loft(&sections).unwrap();
    let wall_count = reference.faces.len() - 2;
    for face in &mut reference.0.faces[..wall_count] {
        let surface = &mut face.surface;
        if multispan && !(surface.control_points[0][0][1] == 0.
            && surface.control_points[1][0][1] == 0.) {
            continue;
        }
        let start = surface.control_points[0][0].clone();
        let end = surface.control_points[1][1].clone();
        surface.degree_u = 2;
        surface.degree_v = 4;
        surface.knots_u = [vec![0.; 3], vec![1.; 3]].concat();
        surface.knots_v = [vec![0.; 5], vec![1.; 5]].concat();
        surface.control_points = (0..=2)
            .map(|u| {
                let f = u as f64 * 0.5;
                (0..=4)
                    .map(|v| {
                        vec![
                            start[0] + f * (end[0] - start[0]),
                            start[1] + f * (end[1] - start[1]),
                            start[2] + v as f64,
                        ]
                    })
                    .collect()
            })
            .collect();
        surface.weights = vec![vec![1.; 5]; 3];
        surface.weights[1][2] = 4.;
        // Choose the central height so the homogeneous quotient has
        // zero station acceleration on the internal side of each patch.
        // The transverse acceleration agrees because the x/y net and
        // weights are symmetric across the two station endpoints.
        surface.control_points[1][2][2] = if start[2] == 0. { 3.5 } else { 4.5 };
    }
    if curved_seam {
        // Bend the shared station seam on the y=0 wall. The upper
        // chart has a nonconvex projected image above this bowed edge.
        // Move the same three seam-adjacent poles on each side to
        // retain matching first and second seam jets.
        let bump = 0.125;
        for face in &mut reference.0.faces[..wall_count] {
            let s = &mut face.surface;
            if s.control_points[0][0][1] == 0. && s.control_points[s.degree_u][0][1] == 0. {
                let columns = if s.control_points[0][0][2] == 0. { 2..5 } else { 0..3 };
                for j in columns {
                    s.control_points[1][j][2] += bump;
                }
            }
        }
        for edge in &mut reference.0.edges {
            let c = &mut edge.curve;
            let [a, b] = c.control_points.as_slice() else { continue };
            if a[1] == 0. && b[1] == 0. && a[2] == 4. && b[2] == 4. {
                let middle = vec![(a[0] + b[0]) / 2., 0., 4. + bump];
                c.control_points.insert(1, middle);
                c.degree = 2;
                c.knots = vec![0., 0., 0., 1., 1., 1.];
                c.weights = vec![1.; 3];
            }
        }
    }
    if oblique {
        reference = crate::transform::affine(&reference, [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0.25, 0.25, 1., 0.],
            [0., 0., 0., 1.],
        ]).unwrap();
    }
    if multispan {
        // Clear the odd factors of the new homogeneous weights so every
        // inserted Cartesian coordinate stays exactly representable.
        let axial_scale = if insertions == 1 { 5. } else { 5. * 7. * 17. };
        reference = crate::transform::affine(&reference, [
            [1., 0., 0., 0.], [0., 1., 0., 0.],
            [0., 0., axial_scale, 0.], [0., 0., 0., 1.],
        ]).unwrap();
        let surface = &mut reference.0.faces[4].surface;
        let inserted = nurbs_core::editing::exact_surface_knot_insertion::insert(
            surface, nurbs_core::surface::Axis::V, 0.5, insertions, 1_000_000,
        ).unwrap();
        assert_eq!(inserted.inserted, insertions, "{:?}", inserted.refusal);
        *surface = inserted.surface;
        assert_eq!(surface.knots_v.iter().filter(|&&k| k == 0.5).count(), insertions);
    }
    reference.validate().unwrap();
    let wall = &reference.faces[if curved_seam { 4 } else { 0 }].surface;
    let chart = nurbs_core::surface_injectivity::certify_chart(wall, 1000).unwrap();
    assert!(chart.proven, "{chart:?}");
    assert_eq!(chart.reason, if multispan {
        assert!(wall.evaluate(0.5, 0.).unwrap().point[2] > 20.);
        "global-rational-multispan-jordan-jacobian"
    } else if curved_seam {
        assert!(wall.evaluate(0.5, 0.).unwrap().point[2] > 4.);
        "global-rational-jordan-jacobian"
    } else if oblique {
        "global-rational-convex-quadrilateral-jacobian"
    } else {
        "global-rational-rectangle-jacobian"
    });
    assert!(!nurbs_core::surface_injectivity::certify_chart(wall, chart.spans - 1).unwrap().proven);

    if insertions == 4 {
        // Move an interior pole out of the shared plane without modifying
        // any edge. The two original walls still partition the XZ image.
        let mut spatial = reference.clone();
        spatial.0.faces[4].surface.control_points[1][6][1] = 0.125;
        spatial.validate().unwrap();
        let certify = crate::shared_boundary::jordan_partition::certify;
        let (contact, used) = certify(&spatial, [0, 4], 1000);
        let contact = contact.unwrap();
        assert_eq!(contact.edges.len(), 1);
        assert!(certify(&spatial, [0, 4], used - 1).0.is_none());
        check_reversed_multispan_exterior(&spatial, contact.edges[0]);
        check_partitioned_jordan_seam(&spatial, contact.edges[0]);
        check_jordan_contact_with_interior_hole(&spatial, contact.edges[0]);
        check_audited_inset_jordan_contact(&spatial, contact.edges[0]);

        // Put the original XZ image in XY, then move the interiors along
        // a common diagonal. The authored edge curves stay unchanged.
        let mut diagonal = crate::transform::affine(&reference, [
            [1., 0., 0., 0.], [0., 0., 1., 0.],
            [0., -1., 0., 0.], [0., 0., 0., 1.],
        ]).unwrap();
        for (face, knot_control) in [(0, 2), (4, 4)] {
            let surface = &mut diagonal.0.faces[face].surface;
            for coordinate in &mut surface.control_points[1][knot_control] {
                *coordinate += 8192.;
            }
            let report = nurbs_core::surface_injectivity::certify_chart(surface, 10_000).unwrap();
            assert_eq!(report.reason, "global-exact-oblique-jordan-jacobian", "{report:?}");
        }
        diagonal.validate().unwrap();
        let (contact, used) = certify(&diagonal, [0, 4], 10_000);
        assert_eq!(contact.unwrap().edges.len(), 1);
        assert!(certify(&diagonal, [0, 4], used - 1).0.is_none());
        check_projection_from_either_face(&reference);
    }

    let mut retained = reference.clone();
    for face in &mut retained.0.faces[..wall_count] {
        for weight in face.surface.weights.iter_mut().flatten() {
            *weight *= 2.;
        }
    }
    let full = inspect(&reference, &retained, 1e-8, 0., limits()).unwrap();
    assert!(full.continuous_bound && full.error_upper == Some(0.));
    assert!(full.certified, "{:?}; volumes {:?}",
        full.reason, full.reference_volume.as_ref().map(|v| (
            v.boundary.agreement.all_equal,
            v.boundary.intersections.faces.all_faces_injective,
            v.boundary.intersections.pairs.all_pairs_classified,
            &v.boundary.intersections.faces,
            v.boundary.intersections.pairs.pairs.iter()
                .filter(|pair| pair.reason == "pair-unresolved")
                .map(|pair| (pair.faces, pair.source_chart_cells, pair.hull_cells))
                .collect::<Vec<_>>(),
        )));
    let caps = [wall_count, wall_count + 1];
    let station = crate::sweep_smoothness::inspect_axis(
        &retained, &caps, crate::sweep_smoothness::Axis::Station, 2_000_000,
    ).unwrap();
    assert!(station.extraction_complete && station.g1_certified && station.g2_certified,
        "complete={} g1={} g2={}", station.extraction_complete, station.g1_certified, station.g2_certified);
    assert_eq!(station.edge_ids.len(), 4);
}

fn check_jordan_contact_with_interior_hole(reference: &crate::Model, shared: usize) {
    use nurbs_core::curve_surface_agreement::{lift, verify_exact};
    let mut model = reference.clone();
    // The only nonunit weight is an interior pole. Setting it to one
    // leaves the authored boundaries unchanged and gives dyadic lifts
    // for the four interior isoparametric cuts.
    model.0.faces[0].surface.weights[1][2] = 1.;
    let corners = [[0.25, 0.25], [0.25, 0.5], [0.5, 0.5], [0.5, 0.25]];
    let mut coedges = Vec::new();
    let first_vertex = model.vertices.len();
    for i in 0..4 {
        let pcurve = crate::line(corners[i].to_vec(), corners[(i + 1) % 4].to_vec());
        let surface = &model.faces[0].surface;
        let curve = lift(&pcurve, surface, None, 1e-8, 10_000).unwrap().curve.unwrap();
        let proof = verify_exact(&curve, &pcurve, surface, false, 1_000_000).unwrap().unwrap();
        assert_eq!(proof.outcome, cad_predicates::BezierIdentity::Equal);
        let pole = &curve.control_points[0];
        model.0.vertices.push(crate::Vertex { point: [pole[0], pole[1], pole[2]] });
        let edge = model.edges.len();
        model.0.edges.push(crate::Edge {
            curve,
            vertices: [first_vertex + i, first_vertex + (i + 1) % 4],
            degenerate: false,
        });
        coedges.push(crate::Coedge::new(edge, false, pcurve));
    }
    let hole = model.loops.len();
    model.0.loops.push(crate::Loop { coedges });
    model.0.faces[0].holes.push(hole);
    model.rebuild_topology_ids();
    let certify = crate::shared_boundary::jordan_partition::certify;
    let (contact, used) = certify(&model, [0, 4], 3000);
    assert_eq!(contact.unwrap().edges, [shared]);
    assert!(certify(&model, [0, 4], used - 1).0.is_none());
    model.0.loops[hole].coedges[0].pcurve.control_points[0][0] = 0.;
    assert!(certify(&model, [0, 4], 3000).0.is_none());
}

fn check_audited_inset_jordan_contact(reference: &crate::Model, shared: usize) {
    use crate::face_contacts::{Limits, SharedBoundary};
    let mut lower = reference.faces[0].surface.clone();
    // A unit interior weight makes the exact inset cross-section dyadic.
    lower.weights[1][2] = 1.;
    let upper = &reference.faces[4];
    let mut model = reference.clone();
    model.0.vertices.clear();
    model.0.edges.clear();
    model.0.loops.clear();
    model.0.faces.clear();
    model.0.bodies.clear();

    let mut edge = |curve: crate::Curve| {
        let vertices = [0, curve.control_points.len() - 1].map(|i| {
            let p = &curve.control_points[i];
            let point = [p[0], p[1], p[2]];
            if let Some(index) = model.0.vertices.iter().position(|v| v.point == point) {
                index
            } else {
                let index = model.0.vertices.len();
                model.0.vertices.push(crate::Vertex { point });
                index
            }
        });
        let index = model.0.edges.len();
        model.0.edges.push(crate::Edge { curve, vertices, degenerate: false });
        index
    };
    let common = edge(reference.edges[shared].curve.clone());
    let upper_wire = &reference.loops[upper.outer].coedges;
    let shared_upper = upper_wire.iter().find(|c| c.edge == shared).unwrap();
    let mut lower_uses = Vec::new();
    let corners = [[0., 0.5], [1., 0.5], [1., 1.], [0., 1.]];
    for i in 0..4 {
        let pcurve = crate::line(corners[i].to_vec(), corners[(i + 1) % 4].to_vec());
        let (index, reversed) = if i == 2 {
            (common, !shared_upper.reversed)
        } else {
            let lift = nurbs_core::curve_surface_agreement::lift(
                &pcurve, &lower, None, 1e-8, 10_000,
            ).unwrap();
            (edge(lift.curve.unwrap()), false)
        };
        lower_uses.push(crate::Coedge::new(index, reversed, pcurve));
    }
    let upper_uses = upper_wire.iter().map(|c| {
        let index = if c.edge == shared { common } else { edge(reference.edges[c.edge].curve.clone()) };
        crate::Coedge::new(index, c.reversed, c.pcurve.clone())
    }).collect();
    model.0.loops = vec![crate::Loop { coedges: lower_uses }, crate::Loop { coedges: upper_uses }];
    model.0.faces = [lower, upper.surface.clone()].into_iter().enumerate().map(|(outer, surface)| {
        crate::Face { surface, outer, holes: Vec::new() }
    }).collect();
    model.0.shells = vec![brep_topology::Shell {
        closed: false,
        faces: (0..2).map(|face| brep_topology::FaceUse { face, reversed: false }).collect(),
    }];
    model.rebuild_topology_ids();
    model.validate().unwrap();

    let agreement = crate::boundary_agreement::verify_exact(&model, 1_000_000).unwrap();
    assert!(agreement.all_equal && agreement.all_joins_exact);
    let trim = crate::face_domain::audit_trim_regions(&model, 1e-8, 1000, 10_000, 100_000).unwrap();
    assert!(trim.all_valid && trim.faces.iter().all(|f| f.as_ref().unwrap().winding == [Some(1)]));
    let injectivity = crate::face_injectivity::inspect_with_linear(&model, 10_000, 10_000).unwrap();
    assert!(injectivity.all_faces_injective, "{injectivity:?}");
    let jordan = crate::shared_boundary::jordan_partition::certify_audited;
    assert!(crate::shared_boundary::jordan_partition::certify(&model, [0, 1], 8000).0.is_none());
    let (contact, used) = jordan(&model, [0, 1], 8000);
    assert_eq!(contact.unwrap().edges, [common]);
    assert!(jordan(&model, [0, 1], used - 1).0.is_none());

    let contacts = crate::face_contacts::inspect_with_certificates(
        &model, 1e-8, Limits {
            pairs: 1, cells: 10_000, domain_cells: 10_000,
            cells_per_pair: 8_000, domain_cells_per_pair: 8_000,
        }, &[], &[], true,
    ).unwrap();
    assert!(matches!(&contacts.pairs[0].boundary, Some(SharedBoundary::ExactHull(c)) if c.edges == [common]),
        "{:?}", contacts.pairs[0]);
}

fn check_partitioned_jordan_seam(reference: &crate::Model, shared: usize) {
    use crate::feature_ops::edge_partition::{split, Limits};
    let [a, b] = reference.edges[shared].curve.domain();
    let cuts = [a + (b - a) / 4., a + (b - a) / 2.];
    let partition = split(reference, shared, &cuts, "jordan-chain", Limits::default()).unwrap();
    assert!(partition.exact_world_root_partition(1_000_000).unwrap().is_some());
    let model = partition.model();
    let certify = crate::shared_boundary::jordan_partition::certify;
    let (certificate, used) = certify(model, [0, 4], 3000);
    let mut edges = certificate.unwrap().edges;
    let mut expected = partition.edges().to_vec();
    edges.sort_unstable();
    expected.sort_unstable();
    assert_eq!(edges, expected);
    assert_eq!(edges.len(), 3);
    assert!(certify(model, [0, 4], used - 1).0.is_none());
}

fn check_reversed_multispan_exterior(reference: &crate::Model, shared: usize) {
    let mut refined = reference.clone();
    let edge = [0, 4].into_iter()
        .flat_map(|face| &refined.loops[refined.faces[face].outer].coedges)
        .find(|use_| use_.reversed && use_.edge != shared && refined.edges[use_.edge].curve.degree == 1)
        .unwrap().edge;
    let source = &refined.edges[edge].curve;
    let [start, end] = source.domain();
    let inserted = nurbs_core::editing::exact_knot_insertion::insert(
        source, (start + end) / 2., 1, 1_000_000,
    ).unwrap();
    assert_eq!(inserted.inserted, 1, "{:?}", inserted.refusal);
    refined.0.edges[edge].curve = inserted.curve;
    refined.validate().unwrap();
    let certify = crate::shared_boundary::jordan_partition::certify;
    let (contact, used) = certify(&refined, [0, 4], 1000);
    assert_eq!(contact.unwrap().edges, [shared]);
    assert!(certify(&refined, [0, 4], used - 1).0.is_none());

    // The same linear edge in an unclamped quadratic basis. The active
    // endpoints fall halfway between the exterior and interior poles.
    let original = &reference.edges[edge].curve;
    let a = &original.control_points[0];
    let b = &original.control_points[1];
    let unclamped = crate::Curve {
        degree: 2,
        knots: (-2..=4).map(|i| start + f64::from(i) * (end - start) / 2.).collect(),
        control_points: [-0.25, 0.25, 0.75, 1.25].into_iter()
            .map(|t| (0..3).map(|axis| a[axis] + t * (b[axis] - a[axis])).collect())
            .collect(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let clamped = nurbs_core::editing::exact_curve_merge::clamp_exact_curve(&unclamped, 100_000)
        .unwrap().curve.unwrap();
    assert!(nurbs_core::curve_partition_agreement::verify_exact_bspline_root_partition(
        original, &[clamped], original.domain(), 1_000_000,
    ).unwrap().is_some());
    refined.0.edges[edge].curve = unclamped;
    refined.validate().unwrap();
    let (contact, used) = certify(&refined, [0, 4], 3000);
    assert_eq!(contact.unwrap().edges, [shared]);
    assert!(certify(&refined, [0, 4], used - 1).0.is_none());
    check_trimmed_chart_segments(reference, shared, &[edge]);
    check_trimmed_chart_segments(reference, shared, &[edge, shared]);
}

fn check_trimmed_chart_segments(reference: &crate::Model, shared: usize, edges: &[usize]) {
    let mut model = reference.clone();
    // The active interval [1, 2] is asymmetric inside [0, 4].
    let extend = |c: &crate::Curve| {
        let a = &c.control_points[0];
        let b = &c.control_points[1];
        crate::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1.5, 2., 4., 4., 4.],
            // Greville stations preserve the original affine traversal.
            control_points: [-1., -0.25, 0.75, 2., 3.]
                .map(|t| a.iter().zip(b).map(|(a, b)| a + t * (b - a)).collect())
                .to_vec(),
            weights: vec![1.; 5],
            periodic: false,
        }
    };
    let faces = model.faces.clone();
    for face in faces {
        for c in &mut model.0.loops[face.outer].coedges {
            if !edges.contains(&c.edge) {
                continue;
            }
            let curve_parameter = model.0.edges[c.edge].curve.domain();
            c.pcurve = extend(&c.pcurve);
            *c = c.clone().with_trim(brep_topology::CoedgeTrim {
                curve_parameter,
                pcurve_parameter: [1., 2.],
                periodic_lift: [[0; 2]; 2],
            }).unwrap();
        }
    }
    model.validate().unwrap();
    let exact = crate::boundary_agreement::verify_exact(&model, 1_000_000).unwrap();
    assert!(exact.all_equal && exact.all_joins_exact);
    let certify = crate::shared_boundary::jordan_partition::certify;
    let (contact, used) = certify(&model, [0, 4], 4000);
    assert_eq!(contact.unwrap().edges, [shared]);
    assert!(certify(&model, [0, 4], used - 1).0.is_none());
    if edges.contains(&shared) {
        let outer = model.faces[0].outer;
        let c = model.0.loops[outer].coedges.iter_mut().find(|c| c.edge == shared).unwrap();
        let mut trim = c.trim().unwrap();
        trim.curve_parameter[1] = (trim.curve_parameter[0] + trim.curve_parameter[1]) / 2.;
        *c = c.clone().with_trim(trim).unwrap();
        assert!(certify(&model, [0, 4], 4000).0.is_none());
    }

}

fn check_projection_from_either_face(reference: &crate::Model) {
    let mut spatial = crate::transform::affine(reference, [
        [1., 0., 0., 0.], [0., 1., 2., 0.],
        [0., 0., 1., 0.], [0., 0., 0., 1.],
    ]).unwrap();
    // XY is the preferred projection of the lower wall. A displacement
    // at an interior pole folds XY on the upper wall but preserves XZ.
    spatial.0.faces[4].surface.control_points[1][6][1] -= 32_768.;
    spatial.validate().unwrap();
    use nurbs_core::surface_injectivity::{certify_chart, certify_jordan_projection};
    let lower = &spatial.faces[0].surface;
    let upper = &spatial.faces[4].surface;
    assert_eq!(certify_chart(lower, 10_000).unwrap().projection, Some([0, 1]));
    assert_eq!(certify_chart(upper, 10_000).unwrap().projection, Some([0, 2]));
    assert!(!certify_jordan_projection(upper, [[1, 0, 0], [0, 1, 0]], 10_000).unwrap().0);

    let certify = crate::shared_boundary::jordan_partition::certify;
    let (forward, used) = certify(&spatial, [0, 4], 10_000);
    let (reverse, reverse_used) = certify(&spatial, [4, 0], 10_000);
    let [forward, reverse] = [forward.unwrap(), reverse.unwrap()];
    assert_eq!(forward.faces, [0, 4]);
    assert_eq!(reverse.faces, [4, 0]);
    assert_eq!(forward.edges, reverse.edges);
    assert_eq!(forward.contact_enclosure, reverse.contact_enclosure);
    assert_eq!(used, reverse_used);
    for faces in [[0, 4], [4, 0]] {
        assert!(certify(&spatial, faces, used - 1).0.is_none());
    }
}
