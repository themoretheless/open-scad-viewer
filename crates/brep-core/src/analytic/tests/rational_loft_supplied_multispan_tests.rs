    use super::*;
    fn fixture()->(Vec<Vec<Vec<Curve>>>,Vec<Vec<Surface>>){
        let base=crate::sketch::polygon_wire(vec![[0.,0.],[1.,0.],[1.,1.],[0.,1.]]).unwrap();
        let sections=(0..3).map(|z|vec![base.iter().cloned().map(|mut c|{for p in &mut c.control_points{p.push(z as f64);}c}).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let mut sides=vec![Vec::new()];
        for layer in 0..2{for c in &sections[layer][0]{
            let poles=c.control_points.iter().map(|p|vec![p.clone(),vec![p[0]+0.125,p[1],p[2]+0.25],vec![p[0]-0.125,p[1],p[2]+0.75],vec![p[0],p[1],p[2]+1.]]).collect::<Vec<_>>();
            sides[0].push(Surface{degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],control_points:poles,weights:vec![vec![1.;4];2],periodic_u:false,periodic_v:false});
        }}
        (sections,sides)
    }
    #[test]
    fn retains_each_actual_cubic_wall_and_its_owned_edges(){
        let (sections,sides)=fixture();let model=section_loft_surfaces(&sections,&sides,false).unwrap();
        assert_eq!(model.faces.len(),10);
        for (index,face) in model.faces[..8].iter().enumerate(){assert_eq!(face.surface,sides[0][index]);}
        assert!(model.edges.iter().any(|edge|edge.curve.degree==3));
        model.validate().unwrap();
        let mut incomplete=sides.clone();incomplete[0].pop();assert!(section_loft_surfaces(&sections,&incomplete,false).is_err());
        assert!(section_loft_surfaces(&sections,&sides,true).is_err());
    }
