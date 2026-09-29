use value_codec::json;
fn curved_pair() -> brep_core::Model {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let faces = [0, 2];
    let old = brep_core::shared_boundary::inspect_pair(&m, faces)
        .unwrap()
        .unwrap();
    let edge = m.edges[old.edge].curve.clone();
    let mut bowed = edge.clone();
    bowed.degree = 2;
    bowed.knots = vec![0., 0., 0., 1., 1., 1.];
    bowed.weights = vec![1.; 3];
    let mut mid: Vec<_> = edge.control_points[0]
        .iter()
        .zip(&edge.control_points[1])
        .map(|(a, b)| (a + b) * 0.5)
        .collect();
    // Bow along the sum of inward face directions. The bisector plane
    // contains the resulting curved edge and separates both face interiors.
    let mut prepared = Vec::new();
    for face in faces {
        let f = &m.faces[face];
        let c = m.loops[f.outer]
            .coedges
            .iter()
            .find(|c| c.edge == old.edge)
            .unwrap();
        let (axis, row) = {
            let axis = usize::from(c.pcurve.control_points[0][0] != c.pcurve.control_points[1][0]);
            let knots = if axis == 0 {
                &f.surface.knots_u
            } else {
                &f.surface.knots_v
            };
            (
                axis,
                usize::from(c.pcurve.control_points[0][axis] != knots[0]),
            )
        };
        let at = |r: usize| {
            if axis == 0 {
                f.surface.control_points[r][0].clone()
            } else {
                f.surface.control_points[0][r].clone()
            }
        };
        let p = at(row);
        let q = at(1 - row);
        for k in 0..3 {
            mid[k] += (q[k] - p[k]) * 0.125;
        }
        prepared.push((face, axis, row));
    }
    bowed.control_points = vec![
        edge.control_points[0].clone(),
        mid,
        edge.control_points[1].clone(),
    ];
    for (face, axis, row) in prepared {
        let s = &mut m.faces[face].surface;
        let original = s.control_points.clone();
        let start = if axis == 0 {
            &original[row][0]
        } else {
            &original[0][row]
        };
        let reverse = *start != edge.control_points[0];
        let sizes = if axis == 0 { [2, 3] } else { [3, 2] };
        s.control_points = (0..sizes[0])
            .map(|u| {
                (0..sizes[1])
                    .map(|v| {
                        let (r, t) = if axis == 0 { (u, v) } else { (v, u) };
                        if r == row {
                            return bowed.control_points[if reverse { 2 - t } else { t }].clone();
                        }
                        let (p, q) = if axis == 0 {
                            (&original[r][0], &original[r][1])
                        } else {
                            (&original[0][r], &original[1][r])
                        };
                        (0..3)
                            .map(|k| p[k] + (q[k] - p[k]) * t as f64 * 0.5)
                            .collect()
                    })
                    .collect()
            })
            .collect();
        s.weights = vec![vec![1.; sizes[1]]; sizes[0]];
        if axis == 0 {
            s.degree_v = 2;
            s.knots_v = bowed.knots.clone();
        } else {
            s.degree_u = 2;
            s.knots_u = bowed.knots.clone();
        }
    }
    m.edges[old.edge].curve = bowed;
    m.validate().unwrap();
    m
}
fn main() {
    let normal = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut crossing = normal.clone();
    for (f, face) in crossing.faces.iter_mut().enumerate() {
        face.surface.control_points = (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        if f == 1 {
                            vec![u as f64, 0.5, v as f64 - 0.5]
                        } else {
                            vec![u as f64, v as f64, f as f64 * 10.]
                        }
                    })
                    .collect()
            })
            .collect();
    }
    let mut cases = Vec::new();
    for (name, model) in [
        ("cube", normal),
        ("crossing-diagnostic-surfaces", crossing),
        ("curved-shared-boundary", curved_pair()),
    ] {
        let request = json!({"op":"cad_face_contacts","model":model,"toleranceUv":1e-8,"maxPairs":100,"maxCells":10000,"maxDomainCells":100000,"cellsPerPair":16,"domainCellsPerPair":1000,"maxBoxes":2});
        let result = geometry_bridge::dispatch(request.clone()).unwrap();
        let mut partial = request.clone();
        partial["maxPairs"] = json!(1);
        partial["maxBoxes"] = json!(0);
        let partial_result = geometry_bridge::dispatch(partial.clone()).unwrap();
        let display = if name == "curved-shared-boundary" {
            Some(
                geometry_bridge::dispatch(
                    json!({"op":"brep_nurbs_tessellate","model":model,"segments":16}),
                )
                .unwrap(),
            )
        } else {
            None
        };
        cases.push(json!({"name":name,"request":request,"result":result,"partialRequest":partial,"partialResult":partial_result,"display":display}));
    }
    println!("{}", json!({"cases":cases}));
}
