
    use super::*;
    fn profile()->Curve {Curve {degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![0.5,-0.25,0.],vec![1.5,-0.25,0.],vec![2.,0.,0.]],
        weights:vec![1.,0.75,1.25,1.],periodic:false}}
    #[test]
    fn complete_body_partition_bounds_extraction_and_refuses_partial_or_variable_basis(){
        let a=profile();let mut b=a.clone();for p in &mut b.control_points {p[2]=10.;}
        let sections=vec![vec![vec![a]],vec![vec![b]]];
        let (retained,bound,products)=retained_section_partition(&sections,36).unwrap();
        assert_eq!(products,36);assert!(bound.unwrap()>0. && bound.unwrap()<1e-10);
        assert_eq!(retained[0][0].len(),2);
        let (_,partial,products)=retained_section_partition(&sections,35).unwrap();
        assert!(partial.is_none());assert_eq!(products,27);
        let mut changed=sections.clone();changed[1][0][0].weights[1]=0.8;
        assert!(retained_section_partition(&changed,36).unwrap().1.is_none());
        let mut source=sections.clone();source[1][0][0].control_points[1][2]+=0.125;
        assert_ne!(retained_section_partition(&source,36).unwrap().0,retained);
    }
    #[test]
    fn rational_line_arc_length_body_composes_actual_walls_and_filled_caps(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing}};
        let corners=[[0.,0.,0.],[1.,0.,0.],[1.,1.,0.],[0.,1.,0.]];
        let loops=vec![(0..4).map(|i|line(corners[i],corners[(i+1)%4]).unwrap()).collect()];
        let path=Curve {weights:vec![1.,4.],..line([0.;3],[0.,0.,10.]).unwrap()};
        let scale=nurbs_core::progressive_sweep::constant_vector_law([1.,0.,0.]).unwrap();
        let twist=nurbs_core::progressive_sweep::constant_vector_law([0.;3]).unwrap();
        for orientation in [Orientation::Fixed,Orientation::RotationMinimizing] {
            let result=progressive_profile_body_with_evidence(&loops,&path,&scale,&twist,None,None,None,
                Options {normal:[1.,0.,0.],orientation,
                    spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                    initial_sections:3,max_sections:3,max_deviation:0.01}).unwrap();
            assert!(result.approximation.levels.last().unwrap().continuous_bound);
            assert!(result.retained_walls.certified);
            assert!(result.retained_caps.as_ref().unwrap().exact);
            assert!(result.filled_cap_error_upper.is_some());
            assert_eq!(result.boundary_error_within_budget,Some(true));
            assert!(result.boundary_error_upper.unwrap()<=0.01);
        }
    }
    #[test]
    fn curved_arc_length_frame_modes_hollow_body_composes_walls_caps_and_source_domain(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[2.,0.,0.],[2.,2.,0.],[0.,2.,0.]]),
            ring([[0.5,0.5,0.],[0.5,1.5,0.],[1.5,1.5,0.],[1.5,0.5,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::Fixed,false),(Orientation::Fixed,true),(Orientation::FixedNormal,false)] {
        let budget=if orientation==Orientation::FixedNormal {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::FixedNormal {17}else{9},max_deviation:budget},if orientation==Orientation::FixedNormal {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        }
    }
    #[test]
    fn small_fixed_normal_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::FixedNormal,false)] {
        let budget=if orientation==Orientation::FixedNormal {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::FixedNormal {17}else{9},max_deviation:budget},if orientation==Orientation::FixedNormal {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_planar_rmf_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_nonaxial_planar_rmf_arc_length_hollow_body_requires_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let mut loops:Vec<Vec<Curve>>=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        for ring in &mut loops {for curve in ring {for pole in &mut curve.control_points {*pole=vec![pole[0],pole[0],pole[1]];}}}
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.5,-0.5,0.],vec![1.,-1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,1.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        eprintln!("nonaxial RMF body boundary={:?}, filled_caps={:?}, correction={correction}, faces={}",result.boundary_error_upper,result.filled_cap_error_upper,result.model.faces.len());
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_guided_curved_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},Some((&guide,None)),
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_contact_curved_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[1.,0.,0.],[1.,0.1,0.],[0.9,0.1,0.],[0.9,0.,0.]]),
            ring([[0.925,0.025,0.],[0.925,0.075,0.],[0.975,0.075,0.],[0.975,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},Some((&guide,Some((0,0.)))),
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true),"wall={:?},filled={:?},boundary={:?},projection={:?}",result.approximation.levels.last().unwrap().continuous_error_upper,result.filled_cap_error_upper,result.boundary_error_upper,result.cap_projection.as_ref().and_then(|p|p.normal_dots));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn closed_concentric_contact_hollow_body_requires_original_complete_boundary() {
        use nurbs_core::{primitives::circle,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let outer=circle([4.,0.,0.],[0.,1.,0.],0.25).unwrap();
        let hole=circle([4.,0.,0.],[0.,-1.,0.],0.125).unwrap();
        let path=circle([0.;3],[0.,0.,1.],4.).unwrap();
        let guide=circle([0.;3],[0.,0.,1.],4.25).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([1.;3]).unwrap();
        let center=constant_vector_law([0.;3]).unwrap();
        for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
        let result=progressive_profile_body_with_evidence_and_correction(&[vec![outer.clone()],vec![hole.clone()]],&path,&scale,&twist,
            Some((&axes,&center)),None,Some((&guide,Some((0,0.)))),Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
            spacing,initial_sections:17,max_sections:65,max_deviation:2.},None).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);
        assert!(result.retained_caps.is_none() && result.filled_cap_error_upper.is_none());
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=2.);
        assert_eq!(result.model.shells.len(),2);
        let limits=crate::volume_validity::Limits {
            boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                    pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
            nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
            orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000};
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,limits,20000,&[],
            crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,max_chart_cells:100000,
                max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,
            volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        assert_eq!(volume.nesting.as_ref().unwrap().parents,Some(vec![None,Some(0)]));
        assert_eq!(volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>(),vec![Some(true),Some(false)]);
        }
    }
    #[test]
    fn small_frenet_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.,0.1,0.],[0.,0.1,0.1],[0.,0.,0.1]]),
            ring([[0.,0.025,0.025],[0.,0.025,0.075],[0.,0.075,0.075],[0.,0.075,0.025]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::Frenet,false)] {
        let budget=if orientation==Orientation::Frenet {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::Frenet {17}else{9},max_deviation:budget},if orientation==Orientation::Frenet {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn fixed_normal_cap_correction_refuses_exhausted_work_and_excess_displacement(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (quantum,max_work) in [(2_f64.powi(-40),0),(1.,1000000)] {
        let orientation=Orientation::FixedNormal;let authored=false;
        let budget=if orientation==Orientation::FixedNormal {2.}else{0.2};
        let outcome=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::FixedNormal {17}else{9},max_deviation:budget},if orientation==Orientation::FixedNormal {Some(EndpointCapCorrection {quantum,tolerance:1e-9,max_work})}else{None});
        let message=match outcome {Ok(_)=>panic!("Unproved correction was published"),Err(e)=>e.to_string()};
        assert!(message.contains("Progressive cap correction refused"),"{message}");
        }
    }

    #[test]
    fn progressive_body_binds_small_unsegmented_profile_to_actual_walls_and_caps(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing}};
        let loops=vec![vec![profile(),line([2.,0.,0.],[2.,2.,0.]).unwrap(),
            line([2.,2.,0.],[0.,2.,0.]).unwrap(),line([0.,2.,0.],[0.,0.,0.]).unwrap()]];
        let path=line([0.;3],[0.,0.,10.]).unwrap();
        let mut scale=path.clone();scale.control_points=vec![vec![1.,0.,0.];2];
        let mut twist=path.clone();twist.control_points=vec![vec![0.;3];2];
        let result=progressive_profile_body_with_evidence(&loops,&path,&scale,&twist,None,None,None,
            Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,
                initial_sections:3,max_sections:3,max_deviation:0.01}).unwrap();
        assert_eq!(result.body_decomposition_products,54);
        assert!(result.body_decomposition_error_upper.unwrap()>0.);
        assert!(result.retained_walls.certified);
        assert!(result.retained_caps.as_ref().unwrap().exact);
        assert!(result.boundary_error_upper.unwrap()>=result.body_decomposition_error_upper.unwrap());
        assert_eq!(result.boundary_error_within_budget,Some(true));
    }
