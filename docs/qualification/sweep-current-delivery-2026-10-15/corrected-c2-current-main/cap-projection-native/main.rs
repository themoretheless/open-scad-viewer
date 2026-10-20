use nurbs_core::{curve::Curve,section_projection::{project,project_authored_axis}};
fn main(){
        use nurbs_core::progressive_sweep::{Sweep,Options,Orientation,Spacing,constant_vector_law};
        let path=Curve{degree:3,knots:vec![0.,0.,0.,0.,0.5,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.16666666666666666,0.,0.],
                vec![0.5,0.16666666666666666,0.],vec![0.8333333333333334,0.6666666666666666,0.5],vec![1.,1.,1.125]],
            weights:vec![1.;5],periodic:false};
        let profiles=[nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.1).unwrap(),
            nurbs_core::primitives::circle([0.;3],[-1.,0.,0.],0.05).unwrap()];
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
            let options=Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],spacing,
                initial_sections:3,max_sections:129,max_deviation:0.05};
            let sections=profiles.iter().map(|profile|Sweep::new(profile,&path,&scale,&twist,options).unwrap().sections_at(17).unwrap()).collect::<Vec<_>>();
            for (station,t) in [(0,0.),(16,1.)]{
                let original=sections.iter().map(|s|s[station].clone()).collect::<Vec<_>>();
                let mut repaired=nurbs_core::circle_sweep_repair::repair(&[original.clone()],2_f64.powi(-40),1e-9,Some(1000000.)).unwrap().sections.unwrap();
                for curves in [original,repaired.remove(0)] {
                let saved=curves.clone();let normal=path.evaluate(t).unwrap().d1.unwrap();
                let axis=constant_vector_law([normal[0],normal[1],normal[2]]).unwrap();
                if station==16 {
                    let coefficients=[-normal[0]/normal[2],-normal[1]/normal[2]];
                    let p=&curves[0].control_points[0];
                    let offset=((p[2]-coefficients[0]*p[0]-coefficients[1]*p[1])/2_f64.powi(-40)).round()*2_f64.powi(-40);
                    assert!(!project(&curves,2,coefficients,offset,2_f64.powi(-40),1e-9,1000000).unwrap().exact_planar,
                        "Regression must exercise the old rounded-quotient refusal");
                }
                let r=project_authored_axis(&curves,&axis,0.,2_f64.powi(-40),1e-9,1000000).unwrap();
                assert!(r.exact_planar,"{spacing:?}/{station}: {}",r.reason);
                assert!(r.displacement_upper.unwrap()<=1e-9);assert!(r.work>0&&r.work<=1000000);
                let short=project_authored_axis(&curves,&axis,0.,2_f64.powi(-40),1e-9,r.work-1).unwrap();
                assert!(!short.exact_planar&&short.curves.is_none());
                assert!(short.work<r.work);
                assert!(!project_authored_axis(&curves,&axis,0.,2_f64.powi(-40),1e-9,0).unwrap().exact_planar);
                let corrected=r.curves.unwrap();
                for (source,retained) in curves.iter().zip(&corrected){
                    assert_eq!(source.degree,retained.degree);assert_eq!(source.knots,retained.knots);
                    assert_eq!(source.weights,retained.weights);assert_eq!(source.periodic,retained.periodic);
                }
                assert_eq!(curves,saved);
                }
            }
        }
    }
