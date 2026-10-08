#[test]
fn dbg_valence3() {
    use brep_core::aag::{Aag, DihedralClass};
    use brep_core::analytic_features::exact_valence3_corner_blend;
    use nurbs_core::foundation::guards::Budget;
    let budget = || Budget::new(10_000_000, 8, 60_000).unwrap();
    let source = brep_core::cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let max = [10., 8., 6.];
    let edges: Vec<usize> = source.edges.iter().enumerate().filter_map(|(index, edge)| {
        let a = source.vertices[edge.vertices[0]].point;
        let b = source.vertices[edge.vertices[1]].point;
        let mid = [0.5*(a[0]+b[0]), 0.5*(a[1]+b[1]), 0.5*(a[2]+b[2])];
        let touches = edge.vertices.iter().any(|&v| { let p = source.vertices[v].point; (0..3).all(|i| (p[i]-max[i]).abs() <= 1e-9) });
        let on = |i: usize, j: usize| (mid[i]-max[i]).abs() <= 1e-9 && (mid[j]-max[j]).abs() <= 1e-9;
        (touches && (on(0,1) || on(0,2) || on(1,2))).then_some(index)
    }).collect();
    let out = exact_valence3_corner_blend(&source, &edges, 1.).unwrap();
    let mut aag = Aag::build(&out.model, &budget()).unwrap();
    aag.attach_face_attrs(&out.model, &budget()).unwrap();
    for n in &aag.nodes {
        let a = n.attrs.as_ref().unwrap();
        let tn: Vec<usize> = n.edges.iter().filter(|&&e| matches!(aag.edges[e].class, DihedralClass::Smooth|DihedralClass::Tangent))
            .flat_map(|&e| aag.edges[e].uses.iter().map(move |u| u.face)).filter(|&f| f != n.face).collect();
        println!("face {} class={:?} radius={:?} fillet_like={} tang={:?}", n.face, a.class, a.radius, a.fillet_like, tn);
    }
}

#[test]
fn dbg_valence3_edges() {
    use brep_core::aag::Aag;
    use brep_core::analytic_features::exact_valence3_corner_blend;
    use nurbs_core::foundation::guards::Budget;
    let budget = || Budget::new(10_000_000, 8, 60_000).unwrap();
    let source = brep_core::cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let max = [10., 8., 6.];
    let edges: Vec<usize> = source.edges.iter().enumerate().filter_map(|(index, edge)| {
        let a = source.vertices[edge.vertices[0]].point;
        let b = source.vertices[edge.vertices[1]].point;
        let mid = [0.5*(a[0]+b[0]), 0.5*(a[1]+b[1]), 0.5*(a[2]+b[2])];
        let touches = edge.vertices.iter().any(|&v| { let p = source.vertices[v].point; (0..3).all(|i| (p[i]-max[i]).abs() <= 1e-9) });
        let on = |i: usize, j: usize| (mid[i]-max[i]).abs() <= 1e-9 && (mid[j]-max[j]).abs() <= 1e-9;
        (touches && (on(0,1) || on(0,2) || on(1,2))).then_some(index)
    }).collect();
    let out = exact_valence3_corner_blend(&source, &edges, 1.).unwrap();
    let aag = Aag::build(&out.model, &budget()).unwrap();
    for e in &aag.edges {
        println!("edge {} faces={:?} class={:?} sign=({:.3},{:.3})", e.edge, e.uses.iter().map(|u|u.face).collect::<Vec<_>>(), e.class, e.sign_min, e.sign_max);
    }
}
