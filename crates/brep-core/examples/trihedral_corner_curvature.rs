//! Joint central/neighbor-strip shape investigation. No document publication.
use brep_core::{region_replacement as r, *};
use nurbs_core::{Error, Result};
fn stock_limits() -> source_box::Limits {
    source_box::Limits {
        operations: 1000000,
        body: source_body_restore::Limits {
            shell: source_shell_restore::Limits {
                regions: source_region_restore::Limits {
                    region: trimmed_face_recipe::Limits {
                        pairs: 100000,
                        region_cells: 100000,
                        domain_cells: 100000,
                        agreement_cells: 100000,
                    },
                    search_cells: 100000,
                    point_checks: 100000,
                    mapping_cells: 100000,
                    driver_cells: 100000,
                    membership_cells: 100000,
                    controls: 100000,
                    winding_cells: 100000,
                    steps: 64,
                },
                exact_work: 100000000,
                driver_cells: 10000,
            },
            embedding: source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 1000,
                spans: 100000,
                linear_cells: 100000,
                pairs: face_contacts::Limits {
                    pairs: 15,
                    cells: 100000,
                    domain_cells: 100000,
                    cells_per_pair: 10000,
                    domain_cells_per_pair: 10000,
                },
                exact_work: 100000000,
                driver_cells: 100000,
            },
            volume: source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.01,
                tolerance_uv: 1e-8,
                cells: 10000,
                spans: 100000,
                domain_cells: 100000,
            },
        },
    }
}
fn main() -> Result<()> {
    let mut handles = Vec::new();
    let mut profile = None;
    let mut bounded = false;
    let mut matched = false;
    let mut quartic = false;
    let mut rim_origin=0usize;
    let mut degree9=false;
    for arg in std::env::args().skip(1) {
        let parse = |s: &str| {
            s.parse::<f64>().map_err(|_| {
                Error::new(
                    "CORNER_PROBE_INPUT",
                    "Expected numeric handle or --profile=fraction",
                )
            })
        };
        if let Some(value)=arg.strip_prefix("--rim-origin=") {
            rim_origin=value.parse::<usize>().map_err(|_|Error::new("CORNER_PROBE_INPUT","Expected rim origin 0, 1 or 2"))?;
            if rim_origin>=3 {return Err(Error::new("CORNER_PROBE_INPUT","Expected rim origin 0, 1 or 2"));}
        } else if arg == "--degree9-joint" {
            degree9=true;
        } else if arg == "--quartic-profile" {
            quartic=true;bounded=true;
        } else if arg == "--matched-profile" {
            matched=true;
            bounded=true;
        } else if arg == "--bounded-profile" {
            bounded = true;
        } else if let Some(p) = arg.strip_prefix("--profile=") {
            profile = Some(parse(p)?);
        } else {
            handles.push(parse(&arg)?);
        }
    }
    if handles.is_empty() {
        handles = vec![1. / 512., 1. / 256., 1. / 128., 1. / 64., 1. / 32.];
    }
    if handles.len() > 16 || handles.iter().any(|h| !h.is_finite() || *h <= 0.) {
        return Err(Error::new(
            "CORNER_PROBE_INPUT",
            "Use at most 16 positive finite handles",
        ));
    }
    let source = cuboid([0.; 3], [10.; 3])?;
    let before = source.clone();
    let budget = stock_limits();
    let stock = source_box::qualify(&source, &budget)?;
    let requested = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| r::EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect::<Vec<_>>();
    let limits = r::StripLimits {
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        max_geometry_work: 10000,
        max_cells: 300000,
        max_exact: 1000000,
        max_coefficients: 2000000,
    };
    if bounded && profile.is_some() {
        return Err(Error::new(
            "CORNER_PROBE_INPUT",
            "Choose only one profile family",
        ));
    }
    let strips = if quartic {
        r::build_trihedral_strips_with_quartic_profile(&source,0,0,&requested,0.1,2.,limits,10.)?
    } else if matched {
        r::build_trihedral_strips_with_matched_profile(&source,0,0,&requested,0.1,2.,limits,10.)?
    } else if bounded {
        r::build_trihedral_strips_with_bounded_profile(
            &source, 0, 0, &requested, 0.1, 2., limits, 10.,
        )?
    } else if let Some(p) = profile {
        r::build_trihedral_strips_with_profile(&source, 0, 0, &requested, 0.1, 2., limits, p)?
    } else {
        r::build_trihedral_strips(&source, 0, 0, &requested, 0.1, 2., limits)?
    };
    println!(
        "inputs stock_mm=10 radii_mm=[0.1,0.2,0.35] profile={profile:?} bounded={bounded} rim_origin={rim_origin} curvature_limit_per_mm=10"
    );
    if bounded {
        for (i, s) in strips.strips.iter().enumerate() {
            let end = &s.patch.ends[usize::from(s.end_order[0] != 0)];
            println!(
                "source_profile strip={i} radius={} degree={} uv={:?} knots={:?} controls={:?} weights={:?}",
                s.radius_mm,
                end.curve.degree,
                end.skin_pcurve.control_points,
                end.curve.knots,
                end.curve.control_points,
                end.curve.weights
            );
        }
    }
    for handle in handles {
        println!("candidate handle_mm={handle} stage=start");
        let c = match r::propose_trihedral_center_at_rim(
            &stock,
            &strips,
            r::CentralLoftOptions {
                handle_mm: handle,
                tolerances: [1e-7, 1e-5, 1e-4],
                tolerance_uv: 1e-8,
                operations: 1000000,
                cells: 100000,
            },
            rim_origin,
        ) {
            Ok(c) => c,
            Err(e) => {
                println!("candidate handle_mm={handle} stage=proposal refused={e:?}");
                continue;
            }
        };
        if bounded {
            let s = c.surface();
            let p = s.degree_u;
            let q = s.degree_v;
            for seam in [0, 2, 4] {
                let pc = &c.pcurves()[seam];
                let u0 = pc.control_points[0][0];
                let u1 = pc.control_points[1][0];
                let row = if pc.control_points[0][1] == s.knots_v[q] {
                    0
                } else {
                    q
                };
                let next = if row == 0 { 1 } else { q - 1 };
                let mut constant = Vec::new();
                let mut narrowest = f64::INFINITY;
                for span in p..s.control_points.len() {
                    if s.knots_u[span] >= s.knots_u[span + 1]
                        || s.knots_u[span] < u0.min(u1)
                        || s.knots_u[span + 1] > u0.max(u1)
                    {
                        continue;
                    }
                    narrowest = narrowest.min(s.knots_u[span + 1] - s.knots_u[span]);
                    let middle = span - p + p / 2;
                    let axes = (0..3)
                        .filter(|&k| {
                            s.control_points[middle][row][k] != s.control_points[middle][next][k]
                        })
                        .collect::<Vec<_>>();
                    if let [axis] = axes.as_slice() {
                        let first = &s.control_points[span - p][row];
                        if (span - p..=span).all(|i| {
                            (0..3).all(|k| k == *axis || s.control_points[i][row][k] == first[k])
                        }) {
                            constant.push(span);
                        }
                    }
                }
                println!(
                    "candidate handle_mm={handle} stage=profile_basis seam={seam} constant_projected_spans={constant:?} min_clock_width={narrowest}"
                );
            }
            for (i,pole) in c.inspect_pole_limits(1./64.,33333)?.iter().enumerate() {
                let r=&pole.report;
                println!("candidate handle_mm={handle} stage=pole end={i} proven={} differential={} quotient={} reason={} cells={} pending={} band={} normal={:?} tensor={:?} angle={:?} radius={:?} lower={:?}",r.proven,r.differential_proven,r.quotient_proven,r.reason,r.cells,r.pending,r.band_normalized,r.normal_coefficient,r.tensor_coefficient,r.fan_angle_upper,r.reference_radius,r.quotient_lower);
            }
            match c.inspect_exterior_jets(100000, 1000000) {
            Err(e)=>println!("candidate handle_mm={handle} stage=exterior refused={e:?}"),
            Ok(reports)=>for (i, report) in reports.iter().enumerate() {
                println!(
                    "candidate handle_mm={handle} stage=exterior seam={i} decision={:?} position={} regular={} normal={:?} tensor={:?} cells={} pending={} profile_unresolved={} source_unresolved={}",
                    report.decision,
                    report.positions.proven,
                    report.regularity_proven,
                    report.normal,
                    report.tensor,
                    report.cells,
                    report.pending,
                    report.profile_unresolved_queries,
                    report.source_unresolved_queries
                );
            }
            }
        }
        if quartic {
            match c.propose_common_denominator_patches(1000000) {
                Ok(patches) => {
                    println!("candidate handle_mm={handle} stage=common_denominator proposal_patches={} degrees={:?} admitted=false", patches.len(), patches.iter().map(|s|s.degree_u).collect::<Vec<_>>());
                    for (i, pair) in patches.windows(2).enumerate() {
                        let left = pair[0].control_points.last().unwrap();
                        let right = &pair[1].control_points[0];
                        let seam = nurbs_core::curve::Curve {
                            degree: 5, knots: [vec![0.;6],vec![1.;6]].concat(),
                            control_points: left.clone(), weights: vec![1.;6], periodic:false,
                        };
                        let u = pair[0].knots_u[pair[0].control_points.len()];
                        let pcurve = nurbs_core::curve::Curve::from_polyline(vec![vec![u,0.],vec![u,1.]])?;
                        let proof = nurbs_core::surface_g2_pcurves::inspect(
                            nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &seam, pcurves: [&pcurve,&pcurve], surfaces: [&pair[0],&pair[1]], reversed: [false,false] },
                            nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 },
                            6000,
                            100000,
                        )?;
                        println!("candidate handle_mm={handle} stage=network_join seam={i} literal_roots={} positions={} shape={:?}",left==right,proof.positions.proven,proof.geometric);
                        let paired = nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(&pair[0],&pair[1],1e-5,1e-4,6000)?;
                        println!("candidate handle_mm={handle} stage=network_paired seam={i} decision={:?} normal={:?} tensor={:?} regular={} cells={} pending={}",paired.decision,paired.normal,paired.tensor,paired.regularity_proven,paired.cells,paired.pending);
                    }
                },
                Err(e) => println!("candidate handle_mm={handle} stage=common_denominator refused={e:?}"),
            }
        }
        if quartic {
            match c.propose_reconciled_patches(1000000) {
                Err(e)=>println!("candidate handle_mm={handle} stage=reconciled refused={e:?}"),
                Ok(patches)=>{
                    println!("candidate handle_mm={handle} stage=reconciled patches={} admitted=false",patches.len());
                    // Point bounds can reject the absolute cap; successful
                    // points can never admit a whole patch or closed body.
                    for (i,s) in patches.iter().enumerate() {
                        let a=s.knots_u[s.degree_u];let b=s.knots_u[s.control_points.len()];
                        let mut worst=0f64;let mut witness=[0.;2];let mut unresolved=0;
                        for u in [0.25,0.5,0.75] {for v in [0.25,0.5,0.75] {
                            let uv=[a+(b-a)*u,v];
                            let r=nurbs_core::surface_shape_operator::rectangle_bounds(s,[[uv[0];2],[uv[1];2]],8)?;
                            let bound=match r.shape.tensor {Some(t)=>nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)?,None=>None};
                            if let Some(k)=bound {if k[0]>worst {worst=k[0];witness=uv;}} else {unresolved+=1;}
                        }}
                        println!("candidate handle_mm={handle} stage=reconciled_cap_witness patch={i} lower={worst} uv={witness:?} exceeds={} unresolved_points={unresolved} whole_patch_proven=false",worst>10.);
                    }
                    for (i,pair) in patches.windows(2).enumerate() {
                        let p=nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(&pair[0],&pair[1],1e-5,1e-4,6000)?;
                        println!("candidate handle_mm={handle} stage=reconciled_join seam={i} decision={:?} normal={:?} tensor={:?} regular={} cells={} pending={}",p.decision,p.normal,p.tensor,p.regularity_proven,p.cells,p.pending);
                    }
                }
            }
        }
        if quartic {
            for relaxation in [0.,0.25,0.5,0.75,1.] {
                match c.propose_fair_patches(1000000,relaxation) {
                    Err(e)=>println!("candidate handle_mm={handle} stage=fair relaxation={relaxation} refused={e:?}"),
                    Ok(patches)=>{
                        println!("candidate handle_mm={handle} stage=fair relaxation={relaxation} patches={} admitted=false",patches.len());
                        for (i,s) in patches.iter().enumerate() {
                            let a=s.knots_u[s.degree_u];let b=s.knots_u[s.control_points.len()];
                            let mut worst=0f64;let mut witness=[0.;2];let mut unresolved=0;
                            for u in [0.25,0.5,0.75] {for v in [0.25,0.5,0.75] {
                                let uv=[a+(b-a)*u,v];
                                let r=nurbs_core::surface_shape_operator::rectangle_bounds(s,[[uv[0];2],[uv[1];2]],8)?;
                                let bound=match r.shape.tensor {Some(t)=>nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)?,None=>None};
                                if let Some(k)=bound {if k[0]>worst {worst=k[0];witness=uv;}} else {unresolved+=1;}
                            }}
                            println!("candidate handle_mm={handle} stage=fair_cap_witness relaxation={relaxation} patch={i} lower={worst} uv={witness:?} exceeds={} unresolved_points={unresolved} whole_patch_proven=false",worst>10.);
                        }
                        for (i,pair) in patches.windows(2).enumerate() {
                            let p=nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(&pair[0],&pair[1],1e-5,1e-4,6000)?;
                            println!("candidate handle_mm={handle} stage=fair_join relaxation={relaxation} seam={i} decision={:?} normal={:?} tensor={:?} regular={} cells={} pending={}",p.decision,p.normal,p.tensor,p.regularity_proven,p.cells,p.pending);
                        }
                    }
                }
            }
        }
        if quartic {
            for relaxation in [0.,0.25,0.5,0.75,1.] {
                match c.propose_bivariate_patches(1000000,relaxation) {
                    Err(e)=>println!("candidate handle_mm={handle} stage=bivariate relaxation={relaxation} refused={e:?}"),
                    Ok(patches)=>{
                        println!("candidate handle_mm={handle} stage=bivariate relaxation={relaxation} patches={} admitted=false",patches.len());
                        for (i,s) in patches.iter().enumerate() {
                            let a=s.knots_u[s.degree_u];let b=s.knots_u[s.control_points.len()];
                            let mut worst=0f64;let mut witness=[0.;2];let mut unresolved=0;
                            for u in [0.25,0.5,0.75] {for v in [0.25,0.5,0.75] {
                                let uv=[a+(b-a)*u,v];
                                let r=nurbs_core::surface_shape_operator::rectangle_bounds(s,[[uv[0];2],[uv[1];2]],8)?;
                                let bound=match r.shape.tensor {Some(t)=>nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)?,None=>None};
                                if let Some(k)=bound {if k[0]>worst {worst=k[0];witness=uv;}} else {unresolved+=1;}
                            }}
                            println!("candidate handle_mm={handle} stage=bivariate_cap_witness relaxation={relaxation} patch={i} lower={worst} uv={witness:?} exceeds={} unresolved_points={unresolved} whole_patch_proven=false",worst>10.);
                        }
                        for (i,pair) in patches.windows(2).enumerate() {
                            let p=nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(&pair[0],&pair[1],1e-5,1e-4,6000)?;
                            println!("candidate handle_mm={handle} stage=bivariate_join relaxation={relaxation} seam={i} decision={:?} normal={:?} tensor={:?} regular={} cells={} pending={}",p.decision,p.normal,p.tensor,p.regularity_proven,p.cells,p.pending);
                        }
                    }
                }
            }
        }
        if quartic {
            for sweeps in [0,2,4,6] {
                println!("candidate handle_mm={handle} stage=curvature_search sweeps={sweeps} state=start evaluations_limit={}",if sweeps==0 {0} else {50000});
                let candidate=if sweeps==0 {c.propose_bivariate_patches(1000000,1.)} else {c.propose_curvature_search_patches(1000000,50000,sweeps)};
                match candidate {
                    Err(e)=>println!("candidate handle_mm={handle} stage=curvature_search sweeps={sweeps} refused={e:?}"),
                    Ok(patches)=>{
                        println!("candidate handle_mm={handle} stage=curvature_search sweeps={sweeps} patches={} admitted=false",patches.len());
                        for (i,s) in patches.iter().enumerate() {
                            let a=s.knots_u[s.degree_u];let b=s.knots_u[s.control_points.len()];
                            let mut worst=0f64;let mut witness=[0.;2];let mut unresolved=0;
                            // Independent outward bounds, not the approximate
                            // score used inside the proposal search.
                            for u in [0.125,0.25,0.5,0.75,0.875] {for v in [0.125,0.25,0.5,0.75,0.875] {
                                let uv=[a+(b-a)*u,v];
                                let r=nurbs_core::surface_shape_operator::rectangle_bounds(s,[[uv[0];2],[uv[1];2]],8)?;
                                let bound=match r.shape.tensor {Some(t)=>nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)?,None=>None};
                                if let Some(k)=bound {if k[0]>worst {worst=k[0];witness=uv;}} else {unresolved+=1;}
                            }}
                            println!("candidate handle_mm={handle} stage=search_cap_witness sweeps={sweeps} patch={i} lower={worst} uv={witness:?} exceeds={} unresolved_points={unresolved} whole_patch_proven=false",worst>10.);
                        }
                        for (i,pair) in patches.windows(2).enumerate() {
                            let p=nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(&pair[0],&pair[1],1e-5,1e-4,6000)?;
                            println!("candidate handle_mm={handle} stage=search_join sweeps={sweeps} seam={i} decision={:?} normal={:?} tensor={:?} regular={} cells={} pending={}",p.decision,p.normal,p.tensor,p.regularity_proven,p.cells,p.pending);
                            if sweeps==6 {
                                for (side,s) in pair.iter().enumerate() {
                                    let u=if side==0 {s.knots_u[s.control_points.len()]} else {s.knots_u[s.degree_u]};
                                    let mut worst=0f64;let mut at=0.;let mut unresolved=0;
                                    for v in [0.,0.125,0.25,0.5,0.75,0.875,1.] {
                                        let r=nurbs_core::surface_shape_operator::rectangle_bounds(s,[[u;2],[v;2]],8)?;
                                        let k=match r.shape.tensor {Some(t)=>nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)?,None=>None};
                                        if let Some(k)=k {if k[0]>worst {worst=k[0];at=v;}} else {unresolved+=1;}
                                    }
                                    println!("candidate handle_mm={handle} stage=search_seam_cap_witness seam={i} side={side} lower={worst} uv={:?} exceeds={} unresolved_points={unresolved} whole_seam_cap_proven=false",[u,at],worst>10.);
                                }
                            }
                        }
                    }
                }
            }
        }
        if quartic {
            for sweeps in if degree9 {vec![3]} else {vec![2,4,6]} {
                println!("candidate handle_mm={handle} stage=joint_seam_search sweeps={sweeps} state=start");
                match if degree9 {c.propose_joint_seam_degree9_patches(1000000,95000,sweeps)} else {c.propose_joint_seam_patches(1000000,95000,sweeps)} {
                    Err(e)=>println!("candidate handle_mm={handle} stage=joint_seam_search sweeps={sweeps} refused={e:?}"),
                    Ok(patches)=>{
                        println!("candidate handle_mm={handle} stage=joint_seam_search sweeps={sweeps} patches={} admitted=false",patches.len());
                        if degree9 {
                            match c.sew_network_candidate(&patches,r::CornerSewLimits {cells:100000,exact_work:1000000,trim_pairs:10000,domain_cells:100000}) {
                                Ok(body)=>{
                                    println!("candidate handle_mm={handle} stage=network_body faces={} edges={} vertices={} bodies={} closed={} cells={} final_g2_seams={} trims_valid={} solid_admitted=false",body.model().faces.len(),body.model().edges.len(),body.model().vertices.len(),body.model().bodies.len(),body.model().shells[0].closed,body.cells,body.final_jets().len(),body.trims().all_valid);
                                    for (i,r) in body.final_jets().iter().enumerate() {println!("candidate handle_mm={handle} stage=network_body_g2 seam={i} decision={:?} normal={:?} tensor={:?} regular={} pending={}",r.decision,r.normal,r.tensor,r.regularity_proven,r.pending);}
                                    let uv=[0.5285379208722983,0.125];let r=nurbs_core::surface_shape_operator::rectangle_bounds(&body.model().faces[body.central_faces()[2]].surface,[[uv[0];2],[uv[1];2]],8)?;
                                    if let Some(t)=r.shape.tensor {if let Some(k)=nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)? {println!("candidate handle_mm={handle} stage=network_body_cap witness_lower={} limit=10 admitted=false",k[0]);}}
                                },
                                Err(e)=>println!("candidate handle_mm={handle} stage=network_body refused={e:?}"),
                            }
                            match c.assemble_open_network(&patches,100000,1000000) {
                                Ok(shell)=>{
                                    println!("candidate handle_mm={handle} stage=open_network faces={} edges={} vertices={} internal_edges={} bindings={} bodies={} closed={} cells={}",shell.model().faces.len(),shell.model().edges.len(),shell.model().vertices.len(),shell.shared_edges().len(),shell.support_binding_proposals().len(),shell.model().bodies.len(),shell.model().shells[0].closed,shell.cells);
                                    for binding in shell.support_binding_proposals() {
                                        let proof=nurbs_core::curve_surface_agreement::verify(&shell.model().edges[binding.edge].curve,&binding.pcurve,&binding.support,false,1e-7,10000)?;
                                        println!("candidate handle_mm={handle} stage=network_support_binding boundary={} patch={} edge={} status={:?} cells={} witness={:?} distance={:?}",binding.boundary,binding.patch,binding.edge,proof.status,proof.cells,proof.witness,proof.witness_distance);
                                    }
                                },
                                Err(e)=>println!("candidate handle_mm={handle} stage=open_network refused={e:?}"),
                            }
                            for r in c.inspect_network_exterior_support_jets(&patches,100000,1000000)? {
                                match r.report {
                                    Err(e)=>println!("candidate handle_mm={handle} stage=network_support boundary={} patch={} refused={e:?}",r.boundary,r.patch),
                                    Ok(p)=>println!("candidate handle_mm={handle} stage=network_support boundary={} patch={} clock={:?} position={} decision={:?} regular={} normal={:?} tensor={:?} cells={} pending={} profile_unresolved={} source_unresolved={}",r.boundary,r.patch,r.parameter,p.positions.proven,p.decision,p.regularity_proven,p.normal,p.tensor,p.cells,p.pending,p.profile_unresolved_queries,p.source_unresolved_queries),
                                }
                            }
                            match c.inspect_network_exterior_jets(&patches,100000,1000000) {
                                Err(e)=>println!("candidate handle_mm={handle} stage=network_exterior refused={e:?}"),
                                Ok(reports)=>for r in reports {
                                    println!("candidate handle_mm={handle} stage=network_exterior boundary={} patch={} clock={:?} position={} shape={:?}",r.boundary,r.patch,r.parameter,r.report.positions.proven,r.report.geometric);
                                    for (side,p) in r.report.positions.sides.iter().enumerate() {
                                        println!("candidate handle_mm={handle} stage=network_position boundary={} patch={} side={side} exact={:?} interval={:?}",r.boundary,r.patch,p.exact,p.interval);
                                    }

                                }
                            }
                        }

                        for (i,s) in patches.iter().enumerate() {
                            let a=s.knots_u[s.degree_u];let b=s.knots_u[s.control_points.len()];
                            let mut worst=0f64;let mut witness=[0.;2];let mut unresolved=0;
                            for u in [0.125,0.25,0.5,0.75,0.875] {for v in [0.125,0.25,0.5,0.75,0.875] {
                                let uv=[a+(b-a)*u,v];
                                let r=nurbs_core::surface_shape_operator::rectangle_bounds(s,[[uv[0];2],[uv[1];2]],8)?;
                                let bound=match r.shape.tensor {Some(t)=>nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)?,None=>None};
                                if let Some(k)=bound {if k[0]>worst {worst=k[0];witness=uv;}} else {unresolved+=1;}
                            }}
                            println!("candidate handle_mm={handle} stage=joint_cap_witness sweeps={sweeps} patch={i} lower={worst} uv={witness:?} exceeds={} unresolved_points={unresolved} whole_patch_proven=false",worst>10.);
                        }
                        for (i,pair) in patches.windows(2).enumerate() {
                            let p=nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(&pair[0],&pair[1],1e-5,1e-4,6000)?;
                            println!("candidate handle_mm={handle} stage=joint_join sweeps={sweeps} seam={i} decision={:?} normal={:?} tensor={:?} regular={} cells={} pending={}",p.decision,p.normal,p.tensor,p.regularity_proven,p.cells,p.pending);
                            if sweeps==6 || degree9 {for (side,s) in pair.iter().enumerate() {
                                let u=if side==0 {s.knots_u[s.control_points.len()]} else {s.knots_u[s.degree_u]};
                                let mut worst=0f64;let mut at=0.;let mut unresolved=0;
                                for v in [0.,0.125,0.25,0.5,0.75,0.875,1.] {
                                    let r=nurbs_core::surface_shape_operator::rectangle_bounds(s,[[u;2],[v;2]],8)?;
                                    let k=match r.shape.tensor {Some(t)=>nurbs_core::surface_shape_operator::absolute_spectral_bounds(t)?,None=>None};
                                    if let Some(k)=k {if k[0]>worst {worst=k[0];at=v;}} else {unresolved+=1;}
                                }
                                println!("candidate handle_mm={handle} stage=joint_seam_cap_witness seam={i} side={side} lower={worst} uv={:?} exceeds={} unresolved_points={unresolved} whole_seam_cap_proven=false",[u,at],worst>10.);
                            }}
                        }
                    }
                }
            }
        }
        if bounded {
            for (knot, raw) in c.inspect_internal_span_joins(6000)? {
                let geometric = nurbs_core::surface_g2_pcurves::inspect_original_geometric_span_join(
                    c.surface(), knot, 1e-5, 1e-4, 6000,
                )?;
                println!(
                    "candidate handle_mm={handle} stage=internal knot={knot} raw={:?} geometric={:?} regular={} normal={:?} tensor={:?} cells={} pending={} clocks={:?}",
                    raw.decision, geometric.decision, geometric.regularity_proven,
                    geometric.normal, geometric.tensor, geometric.cells,
                    geometric.pending, geometric.cross_scales
                );
            }
        }
        let sewn = match c.sew_corner(r::CornerSewLimits {
            cells: 100000,
            exact_work: 1000000,
            trim_pairs: 10000,
            domain_cells: 1000000,
        }) {
            Ok(c) => c,
            Err(e) => {
                println!("candidate handle_mm={handle} stage=sewing refused={e:?}");
                continue;
            }
        };
        let g2 = match sewn.qualify_internal_g2(1e-5, 1e-4, 100000) {
            Ok(g) => g,
            Err(e) => {
                println!("candidate handle_mm={handle} stage=internal_g2 refused={e:?}");
                continue;
            }
        };
        let fields = g2.inspect_curvature(
            1e-8,
            face_curvature::Limits {
                cells: 5000,
                domain_cells: 100000,
                shape_cells: 100000,
            },
        )?;
        println!(
            "candidate handle_mm={handle} stage=curvature complete={} within={} cells={} central_face={}",
            fields.complete,
            fields.within_limit,
            fields.cells,
            sewn.central_face()
        );
        for f in &fields.faces {
            if f.face == sewn.central_face()
                || f.report.decision != face_curvature::Decision::Within
            {
                println!(
                    "candidate handle_mm={handle} face={} decision={:?} upper={:?} witness={:?} uv={:?} reason={}",
                    f.face,
                    f.report.decision,
                    f.report.absolute_curvature_upper,
                    f.report.witness_curvature,
                    f.report.witness_uv,
                    f.report.reason
                );
            }
        }
        match sewn.qualify(
            1e-8,
            r::CornerAuditLimits {
                spans: 1000,
                linear_cells: 5000,
                contacts: face_contacts::Limits {
                    pairs: 100,
                    cells: 100000,
                    domain_cells: 1000000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 10000,
                },
                flux: r::FluxLimits {
                    axis: 0,
                    origin: 0.,
                    absolute_error: 1e-4,
                    cells: 10000,
                    spans: 100000,
                    planar_cells: 10000,
                },
            },
        ) {
            Ok(body) => println!(
                "candidate handle_mm={handle} stage=body volume_mm3={:?} removed_mm3={:?} full_curvature_accepted={}",
                body.volume_mm3(),
                body.removed_volume_mm3(),
                fields.within_limit
            ),
            Err(e) => println!("candidate handle_mm={handle} stage=body refused={e:?}"),
        }
        assert_eq!(source, before);
    }
    Ok(())
}
