    use super::*;
    #[test]
    fn retained_multispan_wall_edges_reverse_knots_with_their_controls() {
        let points=[[ -1.,-1.,0.],[1.,-1.,0.],[1.,1.,0.],[-1.,1.,0.]];
        let section=|z:f64|points.iter().enumerate().map(|(i,p)|Curve {
            degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![p[0],p[1],z],vec![points[(i+1)%4][0],points[(i+1)%4][1],z]],
            weights:vec![1.,1.],periodic:false}).collect::<Vec<_>>();
        let sections=vec![vec![section(0.)],vec![section(5.)]];
        let sides=vec![points.iter().enumerate().map(|(i,p)|Surface {
            degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,0.25,1.,1.,1.],
            control_points:[p,&points[(i+1)%4]].iter().map(|p|[0.,1.,3.,5.].iter().map(|z|vec![p[0],p[1],*z]).collect()).collect(),
            weights:vec![vec![1.,0.75,1.5,1.];2],periodic_u:false,periodic_v:false}).collect::<Vec<_>>()];
        let model=section_loft_surfaces(&sections,&sides,false).unwrap();
        assert_eq!(model.faces.len(),6);
        assert_eq!(model.validate().unwrap().boundary_edge_count,0);
        let before=model.clone();
        let exact=crate::boundary_agreement::verify_exact(&model,1_000_000).unwrap();
        assert!(exact.all_equal && exact.all_joins_exact);
        let exhausted=crate::boundary_agreement::verify_exact(&model,1).unwrap();
        assert!(!exhausted.all_equal && exhausted.work<=1);
        let limits=crate::volume_validity::Limits {
            boundary:crate::boundary_embedding::Limits {exact_work:1_000_000,
                trim_pairs:10_000,trim_cells:100_000,trim_domain_cells:1_000_000,spans:1_000,
                contacts:crate::face_contacts::Limits {pairs:10_000,cells:100_000,
                    domain_cells:1_000_000,cells_per_pair:1_000,domain_cells_per_pair:10_000}},
            nesting_pairs:100,nesting_cells:100_000,nesting_domain_cells:1_000_000,
            orientation_cells:100_000,orientation_domain_cells:1_000_000,orientation_spans:100,
        };
        let caps=crate::sweep_cap_contacts::Budgets {max_walls:1024,max_exact_work:1_000_000,
            max_chart_cells:1000,max_trim_pairs:100_000,max_trim_cells:100_000,max_trim_domain_cells:1_000_000};
        let volume=crate::volume_validity::inspect_sweep(&model,1e-8,limits,20_000,&[4,5],caps).unwrap();
        assert!(volume.boundary.agreement.all_equal);
        assert!(volume.boundary.proven,"exact multispan wall boundary must be embedded");
        assert!(volume.proven,"embedded rational box requires nesting and material orientation");
        assert_eq!(model,before);
    }
