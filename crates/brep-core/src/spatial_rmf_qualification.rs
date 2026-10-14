//! Native constructor-owned closed spatial RMF qualification.
#[test]
fn closed_spatial_rmf_retained_tube_has_complete_owned_walls_and_topology() {
    qualify_closed_spatial_rmf(false, false, false);
}

#[test]
fn closed_spatial_rmf_retained_hollow_tube_has_complete_owned_boundary_and_nesting() {
    qualify_closed_spatial_rmf(true, false, false);
}

#[test]
fn closed_spatial_rmf_varying_rational_laws_hollow_boundary() {
    qualify_closed_spatial_rmf(true, true, false);
}

#[test]
fn periodic_spatial_rmf_original_basis_complete_body_boundary() {
    qualify_closed_spatial_rmf(false, false, true);
}

#[test]
fn periodic_spatial_rmf_varying_laws_hollow_boundary() {
    qualify_closed_spatial_rmf(true, true, true);
}

#[test]
fn parameter_spatial_rmf_varying_laws_hollow_complete_boundary() {
    qualify_closed_spatial_rmf_with_spacing(true,true,false,true);
}

#[test]
fn parameter_periodic_spatial_rmf_varying_laws_hollow_complete_boundary() {
    qualify_closed_spatial_rmf_with_spacing(true,true,true,true);
}

#[test]
fn arc_periodic_spatial_rmf_three_disjoint_holes_complete_boundary_and_roles() {
    qualify_closed_spatial_rmf_contours(3,true,true,false);
}

#[test]
fn parameter_periodic_spatial_rmf_three_disjoint_holes_complete_boundary_and_roles() {
    qualify_closed_spatial_rmf_contours(3,true,true,true);
}

fn qualify_closed_spatial_rmf(hollow: bool, varying: bool, periodic: bool) {
    qualify_closed_spatial_rmf_with_spacing(hollow,varying,periodic,false);
}

fn qualify_closed_spatial_rmf_with_spacing(hollow: bool, varying: bool, periodic: bool, parameter:bool) {
    qualify_closed_spatial_rmf_contours(usize::from(hollow),varying,periodic,parameter);
}

fn qualify_closed_spatial_rmf_contours(holes:usize,varying:bool,periodic:bool,parameter:bool) {
    qualify_closed_spatial_rmf_source(holes,varying,periodic,parameter,false,false);
}

#[test]
fn rational_nonuniform_closed_spatial_rmf_hollow_complete_boundary_and_material() {
    qualify_closed_spatial_rmf_source(1,false,false,false,true,false);
}

#[test]
fn antipodal_closed_spatial_rmf_hollow_complete_boundary_and_material() {
    qualify_closed_spatial_rmf_source(1,false,false,false,false,true);
}

fn qualify_closed_spatial_rmf_source(holes:usize,varying:bool,periodic:bool,parameter:bool,rational:bool,antipodal:bool) {
    let hollow=holes>0;
    use nurbs_core::{
        curve::Curve,
        progressive_sweep::{
            constant_vector_law, Level, MultiSweep, Options, Orientation, Spacing,
        },
    };
    let vertices: [[f64; 3]; 4] = [[1., 0., 0.], [0., 1., 1.], [-1., 0., 0.], [0., -1., 0.5]];
    let tangents: [[f64; 3]; 4] = [
        [0., 0.25, 0.25],
        [-0.25, 0., -0.25],
        [0., -0.25, 0.25],
        [0.25, 0.125, -0.25],
    ];
    let mut poles = vec![vertices[0].to_vec()];
    for i in 0..4 {
        let j = (i + 1) % 4;
        poles.push((0..3).map(|k| vertices[i][k] + tangents[i][k]).collect());
        poles.push((0..3).map(|k| vertices[j][k] - tangents[j][k]).collect());
        poles.push(vertices[j].to_vec());
    }
    let mut path = Curve {
        degree: 3,
        knots: vec![
            0., 0., 0., 0., 0.25, 0.25, 0.25, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 1., 1., 1., 1.,
        ],
        control_points: poles,
        weights: vec![1.; 13],
        periodic: false,
    };
    if rational {
        let fractions=[0.,0.125,0.375,0.75,1.];
        for segment in 0..4 {
            let first=segment*3;
            let ratio=4.*(fractions[segment+1]-fractions[segment]);
            for k in 0..3 {
                path.control_points[first+1][k]=path.control_points[first][k]
                    +ratio*(path.control_points[first+1][k]-path.control_points[first][k]);
                path.control_points[first+2][k]=path.control_points[first+3][k]
                    +ratio*(path.control_points[first+2][k]-path.control_points[first+3][k]);
            }
            path.weights[first+1]=1.25;
            path.weights[first+2]=1.25;
        }
        path.knots=vec![0.,0.,0.,0.,0.125,0.125,0.125,0.375,0.375,0.375,0.75,0.75,0.75,1.,1.,1.,1.];
    }
    if periodic {
        let ring = [
            [1., 0., 0.],
            [0.5, 1., 0.5],
            [-0.5, 1., 1.],
            [-1., 0., 0.],
            [-0.5, -1., 0.25],
            [0.5, -1., -0.5],
        ];
        let mut points = ring.iter().map(|p| p.to_vec()).collect::<Vec<_>>();
        points.extend(ring[..3].iter().map(|p| p.to_vec()));
        path = Curve {
            degree: 3,
            knots: (0..13).map(|k| k as f64).collect(),
            control_points: points,
            weights: vec![1.; 9],
            periodic: true,
        };
    }
    if antipodal {
        let directions: [[f64;3];11]=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.],[1.,0.,0.],[0.,1.,0.],
            [-1.,0.,0.],[0.,0.,-1.],[0.,-1.,0.],[-1.,0.,0.],[0.,-1.,0.],[1.,0.,0.]];
        let mut position=[0.;3];let mut points=vec![position.to_vec()];
        for pair in directions.windows(2) {
            points.push((0..3).map(|k|position[k]+pair[0][k]).collect());
            position=std::array::from_fn(|k|position[k]+pair[0][k]+pair[1][k]);
            points.push(position.to_vec());
        }
        let mut knots=vec![0.;3];for i in 1..10 {knots.extend([i as f64;2]);}knots.extend([10.;3]);
        path=Curve {degree:2,knots,weights:vec![1.;21],control_points:points,periodic:false};
    }
    let start = path.evaluate(path.domain()[0]).unwrap();
    let origin = if antipodal {[0.;3]} else if periodic {
        std::array::from_fn(|k| start.point[k])
    } else {
        [1., 0., 0.]
    };
    let normal = if antipodal {[1.,0.,0.]} else if periodic {
        std::array::from_fn(|k| start.d1.as_ref().unwrap()[k])
    } else {
        [0., 1., 1.]
    };
    let profile = nurbs_core::primitives::circle(origin, normal, 0.05).unwrap();
    let mut profiles = vec![profile];
    if holes==1 {
        profiles.push(nurbs_core::primitives::circle(origin, normal.map(|v| -v), 0.02).unwrap());
    }
    if holes>1 {
        let length=normal.iter().map(|v|v*v).sum::<f64>().sqrt();
        let unit=normal.map(|v|v/length);
        let x=[1.-unit[0]*unit[0],-unit[0]*unit[1],-unit[0]*unit[2]];
        let length=x.iter().map(|v|v*v).sum::<f64>().sqrt();
        let x=x.map(|v|v/length);
        let y=[unit[1]*x[2]-unit[2]*x[1],unit[2]*x[0]-unit[0]*x[2],unit[0]*x[1]-unit[1]*x[0]];
        for offset in [[0.02,0.],[-0.02,0.],[0.,0.02]].into_iter().take(holes) {
            let center=std::array::from_fn(|k|origin[k]+offset[0]*x[k]+offset[1]*y[k]);
            profiles.push(nurbs_core::primitives::circle(center,normal.map(|v|-v),0.0075).unwrap());
        }
    }
    let loops = profiles
        .iter()
        .cloned()
        .map(|p| vec![p])
        .collect::<Vec<_>>();
    let law = |start: [f64; 3], middle: [f64; 3]| {
        if varying {
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![start.to_vec(), middle.to_vec(), start.to_vec()],
                weights: vec![1., 2., 1.],
                periodic: false,
            }
        } else {
            constant_vector_law(start).unwrap()
        }
    };
    let scale = law([1.1, 0., 0.], [1.2, 0., 0.]);
    let twist = law([0.125, 0., 0.], [0.2, 0., 0.]);
    let axes = law([1., 1.25, 0.75], [1.2, 1.1, 0.8]);
    let center = law([0.01, -0.02, 0.03], [0.02, 0., 0.01]);
    let stations=if holes>1||rational||antipodal {65}else{129};
    let options = Options {
        normal: if antipodal {[0.,0.,1.]} else {[1., 0., 0.]},
        orientation: Orientation::RotationMinimizing,
        spacing: if parameter {Spacing::Parameter} else {Spacing::ArcLength {
            tolerance: if rational||antipodal {0.001}else{0.0001},
            max_cells: 100000,
        }},
        initial_sections: stations,
        max_sections: stations,
        max_deviation: if antipodal {1.} else if rational {0.4}else{0.25},
    };
    let body = crate::analytic::progressive_profile_body_with_rmf_policy(
        &loops,
        &path,
        &scale,
        &twist,
        Some((&axes, &center)),
        None,
        None,
        options,
        None,
        Some((4096, 100000, 1000000)),
    )
    .unwrap_or_else(|error| {
        let diagnostic = nurbs_core::progressive_sweep::approximate_spatial_rmf_profiles(
            &profiles,
            &path,
            &scale,
            &twist,
            Some((&axes, &center)),
            options,
            4096,
            100000,
            1000000,
        )
        .unwrap();
        panic!(
            "{error:?}; original error report={:?}",
            diagnostic.levels.last()
        );
    });
    let level = Level {
        patches: body.approximation.patches.clone().unwrap(),
        report: body.approximation.levels.last().unwrap().clone(),
    };
    assert!(level.report.accepted && level.report.continuous_bound);
    let regularity = level.certify_retained_regularity(100000).unwrap();
    assert!(regularity.spanwise_regular);
    let transported = MultiSweep::new(&profiles, &path, &scale, &twist, options)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap()
        .sections_at(stations)
        .unwrap();
    let sections = (0..stations)
        .map(|station| {
            transported
                .iter()
                .map(|profile| vec![profile[station].clone()])
                .collect()
        })
        .collect::<Vec<Vec<Vec<Curve>>>>();
    assert_eq!(body.boundary_error_within_budget, Some(true));
    assert!(body.boundary_error_upper.is_some_and(|e| e <= options.max_deviation));
    assert!(body.retained_walls.certified);
    if periodic {
        let mut damaged = path.clone();
        damaged.knots[1] = 1_f64.next_up();
        damaged.validate().unwrap();
        let refused = crate::analytic::progressive_profile_body_with_rmf_policy(
            &loops,
            &damaged,
            &scale,
            &twist,
            Some((&axes, &center)),
            None,
            None,
            options,
            None,
            Some((4096, 100000, 1000000)),
        )
        .err()
        .expect("An inexact periodic seam must not produce a body");
        assert!(refused.message.contains("continuous retained-patch error"));
    }
    let model = body.model;
    model.validate().unwrap();
    assert_eq!(model.shells.len(),holes+1);
    assert_eq!(
        model.bodies[0].inner_shells.len(),
        holes
    );
    assert_eq!(model.bodies.len(), 1);
    if holes>1 {
        if let Ok(directory)=std::env::var("OSV_SWEEP_QUALIFICATION_MODEL_DIR") {
            let name=if parameter {"periodic-spatial-rmf-three-holes-parameter.json"} else {"periodic-spatial-rmf-three-holes-arc.json"};
            std::fs::write(std::path::Path::new(&directory).join(name),value_codec::to_string(&model).unwrap()).unwrap();
        }
    }
    let walls =
        crate::sweep_retained_walls::inspect(&model, &sections, true, 1024, 1000000).unwrap();
    assert!(
        walls.certified && walls.face_coverage_certified && walls.coefficient_family_certified,
        "{walls:?}"
    );
    let volume = crate::volume_validity::inspect_sweep(
        &model,
        1e-8,
        spatial_volume_limits(),
        100000,
        &[],
        crate::sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work: 1000000,
            max_chart_cells: 100000,
            max_trim_pairs: 10000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 1000000,
        },
    )
    .unwrap();
    // Require the separate global proof: topology and retained-wall coverage
    // alone must never substitute for boundary, nesting and orientation evidence.
    assert!(volume.proven && volume.boundary.proven,
        "volume={} boundary={} agreement={}/{} trim={} injectivity={} pairs={} next={:?} individual={} grouped={} cells={} reasons={:?} nesting={:?} orientation={:?}",
        volume.proven,volume.boundary.proven,volume.boundary.agreement.all_equal,volume.boundary.agreement.all_joins_exact,
        volume.boundary.trim.all_valid,volume.boundary.intersections.faces.all_faces_injective,
        volume.boundary.intersections.pairs.all_pairs_classified,volume.boundary.intersections.pairs.next_pair,
        volume.boundary.intersections.pairs.pairs.len(),volume.boundary.intersections.pairs.grouped_pairs,
        volume.boundary.intersections.pairs.cells,
        volume.boundary.intersections.pairs.pairs.iter().filter(|p| !p.hull_disjoint&&p.reason!="pair-disjoint"&&p.reason!="shared-boundary").take(8).map(|p|(p.faces,p.reason)).collect::<Vec<_>>(),
        volume.nesting.as_ref().map(|n|(&n.parents,n.roles_consistent)),
        volume.orientations.iter().map(|o|(o.expected_outward,o.outward)).collect::<Vec<_>>());
    if holes>1 {
        let mut parents=vec![None];parents.extend((0..holes).map(|_|Some(0)));
        assert_eq!(volume.nesting.as_ref().unwrap().parents,Some(parents));
        assert_eq!(volume.nesting.as_ref().unwrap().roles_consistent,Some(true));
    }
    assert!(volume
        .orientations
        .iter()
        .all(|o| o.outward == Some(o.expected_outward)));
    eprintln!(
        "spatial RMF volume proven={} boundary={} nesting={:?} orientations={:?}",
        volume.proven,
        volume.boundary.proven,
        volume.nesting.as_ref().map(|n| n.roles_consistent),
        volume
            .orientations
            .iter()
            .map(|o| o.outward)
            .collect::<Vec<_>>()
    );
    eprintln!(
        "closed spatial RMF hollow={hollow} varying={varying} periodic={periodic} owned faces={} wall work={} error={:?}",
        model.faces.len(),
        walls.exact_work,
        level.report.continuous_error_upper
    );
}


#[test]
fn open_spatial_rmf_corrected_filled_caps_complete_boundary() {
    qualify_open_spatial_rmf(false);
}

#[test]
fn open_spatial_rmf_varying_hollow_corrected_filled_caps_complete_boundary() {
    qualify_open_spatial_rmf(true);
}

#[test]
fn parameter_open_spatial_rmf_varying_hollow_corrected_filled_caps_complete_boundary() {
    qualify_open_spatial_rmf_with_spacing(true,true);
}

fn qualify_open_spatial_rmf(hollow:bool) {
    qualify_open_spatial_rmf_with_spacing(hollow,false);
}

fn qualify_open_spatial_rmf_with_spacing(hollow:bool,parameter:bool) {
    use nurbs_core::progressive_sweep::{constant_vector_law,Options,Orientation,Spacing};
    let path=nurbs_core::paths::bezier(vec![vec![0.,0.,0.],vec![0.,0.,1.],vec![1.,0.,2.],vec![0.,1.,3.],vec![0.,0.,4.],vec![0.,0.,5.]],None).unwrap();
    let profile=nurbs_core::primitives::circle([0.;3],[0.,0.,1.],0.05).unwrap();
    let mut loops=vec![vec![profile]];
    if hollow {loops.push(vec![nurbs_core::primitives::circle([0.;3],[0.,0.,-1.],0.02).unwrap()]);}
    let law=|start:[f64;3],middle:[f64;3]|if hollow {nurbs_core::curve::Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![start.to_vec(),middle.to_vec(),start.to_vec()],weights:vec![1.,2.,1.],periodic:false}}else{constant_vector_law(start).unwrap()};
    let scale=law([1.1,0.,0.],[1.2,0.,0.]);
    let twist=law([0.125,0.,0.],[0.2,0.,0.]);
    let axes=law([1.,1.25,0.75],[1.2,1.1,0.8]);
    let center=law([0.01,-0.02,0.03],[0.02,0.,0.01]);
    let stations=if hollow {127}else{129};
    let faces=if hollow {8*(stations-1)+2}else{4*(stations-1)+2};
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:if parameter {Spacing::Parameter} else {Spacing::ArcLength {tolerance:0.0001,max_cells:100000}},initial_sections:stations,max_sections:stations,max_deviation:0.25};
    let correction=crate::analytic::EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000};
    let body=crate::analytic::progressive_profile_body_with_rmf_policy(&loops,&path,&scale,&twist,
        Some((&axes,&center)),None,None,options,Some(correction),Some((4096,100000,1000000))).unwrap();
    let report=body.approximation.levels.last().unwrap();
    assert!(report.accepted && report.continuous_bound && !report.closed_path);
    assert!(body.retained_walls.certified);
    assert!(body.retained_caps.as_ref().unwrap().exact,"{:?}",body.retained_caps);
    assert!(body.filled_cap_error_upper.is_some(),"{:?}",body.cap_projection);
    assert_eq!(body.boundary_error_within_budget,Some(true));
    let correction_error=body.cap_correction_error_upper.unwrap();
    assert!(correction_error>0. && correction_error<=1e-9);
    assert!(body.boundary_error_upper.unwrap()>=correction_error);
    body.model.validate().unwrap();
    assert_eq!(body.model.faces.len(),faces);
    assert_eq!(body.model.faces.iter().rev().take(2).map(|f|f.holes.len()).collect::<Vec<_>>(),vec![usize::from(hollow);2]);
    let profiles=loops.iter().flatten().cloned().collect::<Vec<_>>();
    let sizes=vec![1;loops.len()];
    let source=nurbs_core::progressive_sweep::MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap()
        .with_affine_laws(&axes,&center).unwrap().with_spatial_rmf_error_limits(4096,100000,1000000).unwrap();
    let planes=source.certify_ideal_endpoint_planes(&sizes,1e-9,10000,100000,1000000).unwrap();
    assert!(planes.endpoint_normals.is_some(),"{planes:?}");
    let short=source.certify_ideal_endpoint_planes(&sizes,1e-9,10000,planes.cells-1,1000000).unwrap();
    assert!(short.endpoint_normals.is_none());
    let caps=[faces-2,faces-1];
    let volume=crate::volume_validity::inspect_sweep(&body.model,1e-8,spatial_volume_limits(),100000,&caps,
        crate::sweep_cap_contacts::Budgets {max_walls:1024,max_exact_work:1000000,max_chart_cells:100000,
            max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
    assert!(volume.proven && volume.boundary.proven,"boundary={} nesting={:?} orientations={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
    eprintln!("open spatial RMF hollow={hollow} faces={faces} full boundary={:?} filled caps={:?} correction={correction_error}",body.boundary_error_upper,body.filled_cap_error_upper);
}

fn spatial_volume_limits()->crate::volume_validity::Limits {
    crate::volume_validity::Limits {
            boundary: crate::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 10000,
                trim_cells: 100000,
                trim_domain_cells: 1000000,
                spans: 1024,
                contacts: crate::face_contacts::Limits {
                    pairs: 20000,
                    cells: 200000,
                    domain_cells: 1000000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 10000,
                },
            },
            nesting_pairs: 10000,
            nesting_cells: 100000,
            nesting_domain_cells: 1000000,
            orientation_cells: 100000,
            orientation_domain_cells: 1000000,
            orientation_spans: 1000,
        }
}
