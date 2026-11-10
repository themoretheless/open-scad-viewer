
use brep_core::Model;
use brep_core::prism::*;
use nurbs_core::curve::Curve;
    #[test]
    fn scaled_retained_loft_preserves_holes_and_rational_sections() {
        let mut hole=brep_core::sketch::circle_wire(1.).unwrap();
        hole=hole.iter().rev().map(Curve::reverse).collect::<nurbs_core::Result<Vec<_>>>().unwrap();
        let loops=vec![brep_core::sketch::circle_wire(3.).unwrap(),hole];
        let before=value_codec::to_value(&loops).unwrap();
        let loft=loft_scaled(&loops,0.,10.,2.,[5.,7.]).unwrap();
        assert_eq!(loft.bodies.len(),1);
        assert_eq!(loft.faces.iter().filter(|face|face.holes.len()==1).count(),2);
        assert_eq!(value_codec::to_value(&loops).unwrap(),before);
        let side=&loft.faces[0].surface;
        let point=side.evaluate(0.37,0.5).unwrap().point;
        let source=loops[0][0].evaluate(0.37).unwrap().point;
        assert!((point[0]-(1.5*source[0]+2.5)).abs()<1e-12);
        assert!((point[1]-(1.5*source[1]+3.5)).abs()<1e-12);
        assert!((point[2]-5.).abs()<1e-12);
        assert!(side.weights.iter().flatten().any(|w| *w!=1.));
        let (step,_,_)=crate::step_interchange_v3::export_step_v9(&loft).unwrap();
        let (restored,_,_)=crate::step_interchange_v3::import_step_v9(&step).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.bodies.len(),1);
        assert_eq!(restored.faces.iter().filter(|face|face.holes.len()==1).count(),2);
        for scale in [0.,-1.,f64::NAN,f64::INFINITY] {
            assert!(loft_scaled(&loops,0.,10.,scale,[0.,0.]).is_err());
        }
        assert!(loft_scaled(&loops,0.,10.,2.,[1e6,0.]).is_err());
    }
