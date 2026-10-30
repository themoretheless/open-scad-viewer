//! Inspect compact C2 control supports at an actual central material witness.
//! Saved proposals require fresh seam, trim, material and whole-solid checks.
use brep_core::region_replacement::{InteriorLift, propose_interior_lift};
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 5, "candidate.json graft.json u v");
    let candidate: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let data: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let model: brep_core::Model = value_codec::from_value(data["model"].clone()).unwrap();
    let face = data["centralFace"].as_u64().unwrap() as usize;
    let uv = [
        args[3].parse::<f64>().unwrap(),
        args[4].parse::<f64>().unwrap(),
    ];
    let s = &model.faces[face].surface;
    assert_eq!(value_codec::to_value(s).unwrap(), candidate["surface"]);
    assert!(s.weights.iter().flatten().all(|w| *w == 1.));
    let u = nurbs_core::curve::basis(s.degree_u, &s.knots_u, s.control_points.len(), uv[0], false)
        .unwrap();
    let v = nurbs_core::curve::basis(
        s.degree_v,
        &s.knots_v,
        s.control_points[0].len(),
        uv[1],
        false,
    )
    .unwrap();
    let point = s.evaluate(uv[0], uv[1]).unwrap().point;
    let mut poles = Vec::new();
    for (i, &a) in u.basis.iter().enumerate() {
        for (j, &b) in v.basis.iter().enumerate() {
            if a * b > 0. {
                poles.push(([i, j], a * b));
            }
        }
    }
    poles.sort_by(|a, b| b.1.total_cmp(&a.1));
    assert!(poles.len() <= 289);
    let mut results = Vec::new();
    let mut saved = false;
    let domain = brep_core::face_domain::FaceDomain::new(&model, face, 1e-8).unwrap();
    for (pole, weight) in poles {
        let support = [
            [s.knots_u[pole[0]], s.knots_u[pole[0] + s.degree_u + 1]],
            [s.knots_v[pole[1]], s.knots_v[pole[1] + s.degree_v + 1]],
        ];
        let classification = domain.classify(support, 1000).unwrap();
        let amount =
            (1e-6 - point.iter().copied().fold(f64::INFINITY, f64::min)).max(1e-6) / weight;
        match propose_interior_lift(&model,face,&[InteriorLift{pole,displacement:[amount;3]}],1e-8,1000) {
            Ok(proposal)=> {
                results.push(value_codec::json!({"pole":pole,"weight":weight,"inside":true,"cells":proposal.domain_cells,"supports":proposal.support_rectangles}));
                if !saved {
                    let mut output=candidate.clone();
                    output["surface"]=value_codec::to_value(&proposal.surface).unwrap();
                    output["sampledJetResidual"]=value_codec::Value::Null;
                    output["localLift"]=value_codec::json!({"pole":pole,"displacement":[amount,amount,amount],"witnessUV":uv,"sourceFace":face,"domainCells":proposal.domain_cells});
                    output["admitted"]=value_codec::json!(false);
                    let path=std::path::Path::new(&args[1]).with_extension("local-lift.json");
                    std::fs::write(&path,value_codec::to_string(&output).unwrap()).unwrap();
                    println!("saved={} pole={pole:?} delta={amount}",path.display());saved=true;
                }
            },
            Err(error)=>results.push(value_codec::json!({"pole":pole,"weight":weight,"inside":false,"reason":format!("{error}"),"support":support,"location":format!("{:?}",classification.location),"classificationReason":format!("{:?}",classification.reason),"classificationCells":classification.cells})),
        }
    }
    let mut interior_box = value_codec::Value::Null;
    let mut box_cells = 0usize;
    if !saved {
        for level in 0..20 {
            let radius = 0.001 * 0.5f64.powi(level);
            let rectangle = [
                [uv[0] - radius, uv[0] + radius],
                [uv[1] - radius, uv[1] + radius],
            ];
            if rectangle[0][0] <= s.knots_u[s.degree_u]
                || rectangle[0][1] >= s.knots_u[s.control_points.len()]
                || rectangle[1][0] <= s.knots_v[s.degree_v]
                || rectangle[1][1] >= s.knots_v[s.control_points[0].len()]
            {
                continue;
            }
            let classification = domain.classify(rectangle, 1000).unwrap();
            box_cells += classification.cells;
            if classification.location == nurbs_core::trim_domain::Location::Inside {
                let needed = [
                    (&s.knots_u, s.degree_u, rectangle[0]),
                    (&s.knots_v, s.degree_v, rectangle[1]),
                ]
                .map(|(knots, degree, bounds)| {
                    (degree + 2).saturating_sub(
                        knots
                            .iter()
                            .filter(|&&k| k >= bounds[0] && k <= bounds[1])
                            .count(),
                    )
                });
                let counts = [
                    s.control_points.len() + needed[0],
                    s.control_points[0].len() + needed[1],
                ];
                interior_box = value_codec::json!({"rectangle":rectangle,"minimumAdditionalKnotsForThisBox":needed,
                    "minimumRefinedCountsForThisBox":counts,"minimumRefinedPolesForThisBox":counts[0]*counts[1],"cells":classification.cells});
                break;
            }
        }
    }
    let report = value_codec::json!({"witnessUV":uv,"witnessPoint":point,"insideRefinementBox":interior_box,"boxClassificationCells":box_cells,"proposalSaved":saved,"checks":results,"perSupportCellLimit":1000,"additionalClassificationCellLimitPerSupport":1000,"maxAttemptedSupports":289,"materialQualified":false,"solidQualified":false});
    let path = std::path::Path::new(&args[1]).with_extension("local-lift-audit.json");
    std::fs::write(path, value_codec::to_string(&report).unwrap()).unwrap();
    println!("proposalSaved={saved}");
}
