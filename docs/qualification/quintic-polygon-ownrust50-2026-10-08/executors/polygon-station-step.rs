//! Native fixture exporter. No WASM provenance is inferred from this executable.
use geometry_bridge::dispatch;
use nurbs_core::curve::Curve;
use value_codec::{Value, json};

fn sections(count: usize, spatial: bool, weighted: bool) -> Vec<Vec<Vec<Curve>>> {
    let snap = |x: f64| (x * 1024.).round() / 1024.;
    let mut sections = Vec::new();
    for station in 0..count - 1 {
        let angle = std::f64::consts::TAU * station as f64 / (count - 1) as f64;
        let (s, c) = angle.sin_cos();
        let z = if spatial {
            0.125 * (2. * angle).sin()
        } else {
            0.
        };
        let dz = if spatial {
            0.25 * (2. * angle).cos()
        } else {
            0.
        };
        let center = [snap(3. * c), snap(3. * s), snap(z)];
        let radial = [snap(c / 8.), snap(s / 8.), 0.];
        let binormal = [snap(-dz * s / 8.), snap(dz * c / 8.), -0.125];
        let points: Vec<Vec<f64>> = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
            .into_iter()
            .map(|(a, b)| {
                (0..3)
                    .map(|k| center[k] + a * radial[k] + b * binormal[k])
                    .collect()
            })
            .collect();
        sections.push(vec![
            (0..4)
                .map(|i| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![points[i].clone(), points[(i + 1) % 4].clone()],
                    weights: if weighted { vec![1., 2.] } else { vec![1., 1.] },
                    periodic: false,
                })
                .collect(),
        ]);
    }
    sections.push(sections[0].clone());
    sections
}

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .unwrap_or("6".into())
        .parse()
        .unwrap();
    assert!([6, 7, 10, 12].contains(&count));
    let mut cases = Vec::new();
    let selection = std::env::args().nth(2).unwrap_or("all".into());
    for spatial in [false, true] {
        for weighted in [false, true] {
            if selection == "spatial-weighted" && !(spatial && weighted) { continue; }
            let request = json!({"op":"brep_nurbs_smooth_polygon_station_body",
                "sections":sections(count,spatial,weighted),"closed":true,
                "quantum":1./1024.,"maxDisplacement":1.,"maxWork":1000000});
            let body = dispatch(request.clone()).unwrap();
            eprintln!("native body {count}/{spatial}/{weighted}: audited");
            assert_eq!(
                body["accepted"],
                json!(true),
                "{count}/{spatial}/{weighted}: station={} volume={} faces={} pairs={} next={} orientations={} unresolved={}",
                body["stationContinuity"], body["volume"]["solidGeometryCertified"],
                body["volume"]["allFacesInjective"],body["volume"]["allPairsClassified"],
                body["volume"]["nextPair"], body["volume"]["orientations"], body["volume"]["unresolvedPairs"]
            );
            let model = body["model"].clone();
            let exported =
                dispatch(json!({"op":"brep_nurbs_export_step_v5","model":model})).unwrap();
            let text = exported["text"].as_str().unwrap();
            let imported = dispatch(json!({"op":"brep_nurbs_import_step_v5","text":text})).unwrap();
            eprintln!("native STEP {count}/{spatial}/{weighted}: exported and imported");
            let wall_samples: Vec<Vec<Value>> = model["faces"].as_array().unwrap().iter().map(|face|
                [0.,0.17,0.5,0.83,1.].into_iter().flat_map(|u|
                    [0.,0.13,0.5,0.87,1.].into_iter().map(move |v| {
                        let sample = dispatch(json!({"op":"surface_evaluate",
                            "surface":face["surface"],"u":u,"v":v})).unwrap();
                        json!({"u":u,"v":v,"point":sample["point"],"du":sample["du"],"dv":sample["dv"]})
                    })).collect()).collect();
            let edge_curves: Vec<Value> = model["edges"].as_array().unwrap().iter().map(|edge| {
                let samples: Vec<Value> = [0.,0.25,0.5,0.75,1.].into_iter().map(|u| {
                    let sample = dispatch(json!({"op":"curve_evaluate","curve":edge["curve"],"u":u})).unwrap();
                    json!({"u":u,"point":sample["point"]})
                }).collect();
                json!({"curve":edge["curve"],"samples":samples})
            }).collect();
            cases.push(
                json!({"stations":count,"spatial":spatial,"weighted":weighted,
                "request":request,"body":body,"export":exported,"import":imported,
                "wallSamples":wall_samples,"edgeCurves":edge_curves}),
            );
        }
    }
    println!(
        "{}",
        json!({"schema":"native-polygon-station-step/1","cases":cases})
    );
}
