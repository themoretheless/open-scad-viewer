//! Offline form search. No topology, G2 or solid admission from these trials.
use brep_core::{face_curvature as c, face_domain::FaceDomain};
use nurbs_core::{curve::basis, surface::Surface};
fn point_weight(s: &Surface, uv: [f64; 2]) -> f64 {
    let u = basis(s.degree_u, &s.knots_u, s.control_points.len(), uv[0], false)
        .unwrap()
        .basis;
    let v = basis(
        s.degree_v,
        &s.knots_v,
        s.control_points[0].len(),
        uv[1],
        false,
    )
    .unwrap()
    .basis;
    let mut den = 0.;
    let mut inner = 0.;
    for i in 0..u.len() {
        for j in 0..v.len() {
            let b = u[i] * v[j] * s.weights[i][j];
            den += b;
            if i >= 3 && i + 3 < u.len() && j >= 3 && j + 3 < v.len() {
                inner += b;
            }
        }
    }
    inner / den
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "source.graft.json output.json");
    let data: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let source: brep_core::Model = value_codec::from_value(data["model"].clone()).unwrap();
    let f = data["centralFace"].as_u64().unwrap() as usize;
    let uv = [0.9430759112081446, 0.4836167378919158];
    let s = &source.faces[f].surface;
    let domain = FaceDomain::new(&source, f, 1e-8).unwrap();
    let before = s.evaluate(uv[0], uv[1]).unwrap().point;
    let direction = std::env::var("OSV_TILE_LIFT_DIRECTION")
        .unwrap_or_else(|_| "1,1,1".into())
        .split(',')
        .map(|s| s.parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    assert!(
        direction.len() == 3
            && direction[1] == 1.
            && direction.iter().all(|x| x.is_finite() && *x >= 0.)
    );
    let mut trials = Vec::new();
    let widths = std::env::var("OSV_TILE_HALF_WIDTHS")
        .unwrap_or_else(|_| "0.001,0.002,0.004,0.008,0.016,0.032".into())
        .split(',')
        .map(|s| s.parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    assert!(widths.len() <= 32 && widths.iter().all(|x| x.is_finite() && *x > 0. && *x < 0.05));
    for half in widths {
        let rect = uv.map(|x| [x - half, x + half]);
        let ownership = domain.classify(rect, 100000).unwrap();
        if ownership.location != nurbs_core::trim_domain::Location::Inside {
            trials.push(value_codec::json!({"halfWidthUV":half,"ownership":format!("{:?}",ownership.location)}));
            continue;
        }
        let mut tile = s
            .trim([rect[0][0], rect[0][1], rect[1][0], rect[1][1]])
            .unwrap();
        let w = point_weight(&tile, uv);
        let delta = (1e-6 - before.into_iter().fold(f64::INFINITY, f64::min)) / w;
        let n = tile.control_points.len();
        let m = tile.control_points[0].len();
        for i in 3..n - 3 {
            for j in 3..m - 3 {
                for k in 0..3 {
                    tile.control_points[i][j][k] += delta * direction[k];
                }
            }
        }
        let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
        model.faces[0].surface = tile.clone();
        let outer = model.faces[0].outer;
        for co in &mut model.loops[outer].coedges {
            for p in &mut co.pcurve.control_points {
                for k in 0..2 {
                    p[k] = rect[k][0] + p[k] * (rect[k][1] - rect[k][0]);
                }
            }
        }
        let result = c::inspect(
            &model,
            0,
            10.,
            1e-8,
            c::Limits {
                cells: 4000,
                domain_cells: 1000000,
                shape_cells: 1000000,
            },
            &[uv],
        );
        let boxproof = brep_core::face_domain::inspect_box_containment(
            &model,
            0,
            [[-1e-7, 10.0000001]; 3],
            1e-8,
            4000,
            100000,
        )
        .unwrap();
        let record = match result {
            Ok(r) => {
                value_codec::json!({"decision":format!("{:?}",r.decision),"reason":r.reason,"upper":r.absolute_curvature_upper,"witness":r.witness_curvature,"witnessUV":r.witness_uv,"cells":r.cells,"shapeCells":r.shape_cells})
            }
            Err(e) => value_codec::json!({"error":format!("{e}")}),
        };
        eprintln!("half={half} curvature={record:?}");
        trials.push(value_codec::json!({"halfWidthUV":half,"ownership":"Inside","delta":delta,"direction":direction,"curvature":record,"box":format!("{:?}",boxproof.decision),"surface":tile,"G2Qualified":false,"solidQualified":false}));
    }
    std::fs::write(&args[2],value_codec::to_string(&value_codec::json!({"trials":trials,"sourceFace":f,"before":before,"scope":"Offline form search; continuous curvature and material checks on rectangular trial only; no topology or seam admission"})).unwrap()).unwrap();
}
