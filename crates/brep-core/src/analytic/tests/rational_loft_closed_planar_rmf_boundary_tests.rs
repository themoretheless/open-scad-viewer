
    use super::*;
    #[test]
    fn closed_planar_rmf_full_turn_body_composes_owned_periodic_walls_without_caps() {
        use nurbs_core::{primitives::{line,circle},progressive_sweep::{Options,Orientation,Spacing}};
        let points=[[4.9,0.,-0.1],[5.1,0.,-0.1],[5.1,0.,0.1],[4.9,0.,0.1]];
        let loops=vec![(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect()];
        let path=circle([0.;3],[0.,0.,1.],5.).unwrap();
        let scale=nurbs_core::progressive_sweep::constant_vector_law([1.,0.,0.]).unwrap();
        let twist=line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
        let result=progressive_profile_body_with_evidence(&loops,&path,&scale,&twist,None,None,None,
            Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
                spacing:Spacing::Parameter,initial_sections:5,max_sections:129,max_deviation:0.01}).unwrap();
        let report=result.approximation.levels.last().unwrap();
        assert!(report.closed_path);
        assert!(report.accepted&&report.continuous_bound);
        assert!(result.retained_caps.is_none()&&result.cap_projection.is_none());
        assert!(result.filled_cap_error_upper.is_none());
        assert!(result.retained_walls.certified);
        assert_eq!(result.model.faces.len(),4*(report.sections-1));
        assert_eq!(result.body_decomposition_error_upper,Some(0.));
        let upper=result.boundary_error_upper.unwrap();
        assert!(upper>0.&&upper<=0.01);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        let agreement=crate::boundary_agreement::verify_exact(&result.model,1000000).unwrap();
        let trim=crate::face_domain::audit_trim_regions(&result.model,1e-8,10000,100000,1000000).unwrap();
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        eprintln!("closed volume proven={}, orientation cells={}, outward={:?}",volume.proven,
            volume.orientation_cells,volume.orientations.iter().map(|s|s.outward).collect::<Vec<_>>());
        let joint=&volume.boundary;
        let unresolved=joint.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved")
            .map(|p|p.faces).collect::<Vec<_>>();
        eprintln!("joint proven={}, hulls={}, visited={}, next={:?}, cells={}, unresolved={:?}",
            joint.proven,joint.hull_contacts.len(),joint.intersections.pairs.pairs.len(),
            joint.intersections.pairs.next_pair,joint.intersections.pairs.cells,unresolved);
        for pair in &unresolved {
            eprintln!("unresolved hull {:?}: {:?}",pair,crate::boundary_hull_contact::certify(&result.model,*pair));
            for &face in pair {eprintln!("unresolved poles {face}: {:?}",result.model.faces[face].surface.control_points);}
        }
        assert!(joint.proven,"closed periodic boundary must classify every contact");
        for pair in [[125,130],[126,129],[190,195],[191,194]] {
            let contact=crate::boundary_hull_contact::certify(&result.model,pair)
                .expect("rotated station corner requires exact projected contact");
            let vertex=contact.vertex.expect("corner contact must own its shared vertex");
            let mut displaced=result.model.clone();
            displaced.vertices[vertex].point[2]=displaced.vertices[vertex].point[2].next_up();
            assert!(crate::boundary_hull_contact::certify(&displaced,pair).is_none(),
                "one ULP displacement cannot retain exact corner ownership: {pair:?}");
            let mut unowned=result.model.clone();
            let face=&result.model.faces[pair[1]];
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for index in 0..unowned.loops[wire].coedges.len() {
                    let mut edge=unowned.edges[unowned.loops[wire].coedges[index].edge].clone();
                    for vertex in &mut edge.vertices {
                        let source=unowned.vertices[*vertex].clone();
                        *vertex=unowned.vertices.len();unowned.vertices.push(source);
                    }
                    unowned.loops[wire].coedges[index].edge=unowned.edges.len();unowned.edges.push(edge);
                }
            }
            assert!(crate::boundary_hull_contact::certify(&unowned,pair).is_none(),
                "coincident poles without shared topology cannot authorize corner contact: {pair:?}");
        }
        assert!(volume.proven,"closed periodic body requires native nesting and outward orientation");
        let winding=trim.faces.iter().filter(|r|r.as_ref().is_some_and(|r|r.winding.iter().all(|w|*w==Some(1)))).count();
        eprintln!("closed body exact={}, joins={}, trim={}, positive winding faces={}",
            agreement.all_equal,agreement.all_joins_exact,trim.all_valid,winding);
        for pair in [[0,5],[0,7],[0,result.model.faces.len()-3],[0,result.model.faces.len()-1]] {
            eprintln!("closed body hull {:?}: {:?}",pair,crate::boundary_hull_contact::certify(&result.model,pair));
        }
    }
