//! Whole-domain RMF source bound for every retained polygon wall.
//! Original quintic patches are concatenated without changing their images.
use crate::{Result, field, input};
use nurbs_core::{curve::Curve, surface::Surface};
use value_codec::{Value, json};

fn concatenate(patches: &[&Surface]) -> Result<Surface> {
    let first = patches
        .first()
        .ok_or_else(|| input("Missing retained wall patches"))?;
    let mut joined = (*first).clone();
    joined.knots_v = vec![0.; 6];
    for station in 1..patches.len() {
        let knot = station as f64 / patches.len() as f64;
        joined.knots_v.extend(std::iter::repeat_n(knot, 5));
    }
    joined.knots_v.extend([1.; 6]);
    for patch in patches.iter().skip(1) {
        if patch.degree_u != 1
            || patch.degree_v != 5
            || patch.knots_u != first.knots_u
            || patch.knots_v != first.knots_v
            || patch.control_points.len() != 2
        {
            return Err(input("Incompatible retained source-wall basis"));
        }
        for row in 0..2 {
            if joined.control_points[row].last() != patch.control_points[row].first()
                || joined.weights[row].last() != patch.weights[row].first()
            {
                return Err(input("Nonexact retained source-wall join"));
            }
            joined.control_points[row].extend(patch.control_points[row].iter().skip(1).cloned());
            joined.weights[row].extend(patch.weights[row].iter().skip(1));
        }
    }
    joined.validate()?;
    Ok(joined)
}

pub(super) fn certify(source: &Value, sides: &[Vec<Surface>], closed: bool) -> Result<Value> {
    let fields = source
        .as_object()
        .ok_or_else(|| input("Expected RMF source object"))?;
    if fields.keys().any(|key| {
        ![
            "profiles",
            "path",
            "scale",
            "normal",
            "maxDeviation",
            "maxCells",
        ]
        .contains(&key.as_str())
    }) {
        return Err(input("Only the declared RMF reference family is supported"));
    }
    let profiles: Vec<Vec<Curve>> = field(source, "profiles")?;
    let path: Curve = field(source, "path")?;
    let scale: Curve = field(source, "scale")?;
    let normal = field(source, "normal")?;
    let budget: f64 = field(source, "maxDeviation")?;
    let max_cells: usize = field(source, "maxCells")?;
    if !budget.is_finite()
        || budget < 0.
        || max_cells > 100000
        || profiles.len() != sides.len()
        || profiles.iter().any(Vec::is_empty)
    {
        return Err(input("Invalid polygon source-bound request"));
    }
    for wire in &profiles {
        if wire.len() < 3 {
            return Err(input("Closed RMF source requires polygon loops"));
        }
        for (edge, curve) in wire.iter().enumerate() {
            curve.validate()?;
            if curve.degree != 1
                || curve.knots != [0., 0., 1., 1.]
                || curve.control_points.len() != 2
                || curve.control_points.last()
                    != wire[(edge + 1) % wire.len()].control_points.first()
            {
                return Err(input(
                    "Closed RMF source requires exact polygon profile joins",
                ));
            }
        }
    }
    // This operation currently bounds all walls of a closed body. Filled caps
    // and alternative guide/authored-frame reference families need their own proof.
    if !closed || !nurbs_core::progressive_sweep::path_is_closed(&path)? {
        return Ok(json!({"withinBudget":false,"errorUpper":null,"cells":0,
            "maxCells":max_cells,"reason":"closed-rmf-source-required"}));
    }
    let mut cells = 0;
    let mut maximum = 0_f64;
    let mut walls = Vec::new();
    let mut complete = true;
    for (l, loop_profiles) in profiles.iter().enumerate() {
        let n = loop_profiles.len();
        if sides[l].is_empty() || sides[l].len() % n != 0 {
            return Err(input("Source profiles do not match retained walls"));
        }
        for (edge, profile) in loop_profiles.iter().enumerate() {
            let patches = sides[l].iter().skip(edge).step_by(n).collect::<Vec<_>>();
            let retained = concatenate(&patches)?;
            let report = nurbs_core::sweeps::profile_certificate::certify(
                profile,
                &path,
                &scale,
                normal,
                &retained,
                budget,
                max_cells - cells,
            )?;
            cells += report.cells;
            complete &= report.within_budget;
            if let Some(error) = report.error_upper {
                maximum = maximum.max(error);
            } else {
                complete = false;
            }
            walls.push(json!({"loop":l,"edge":edge,"errorUpper":report.error_upper,
                "withinBudget":report.within_budget,"cells":report.cells,
                "method":report.method,"reason":report.reason}));
        }
    }
    Ok(
        json!({"method":"retained-quintic-rmf-wall-bound","scope":"all-retained-closed-body-walls",
        "withinBudget":complete,"errorUpper":if complete {Some(maximum)} else {None},
        "cells":cells,"maxCells":max_cells,"budget":budget,"walls":walls}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> (Value, Vec<Vec<Surface>>) {
        let request = super::super::tests::request();
        let sections: Vec<Vec<Vec<Curve>>> = field(&request, "sections").unwrap();
        let candidate = brep_core::analytic::smooth_polygon_station_walls(
            &sections,
            &[],
            true,
            1. / 1024.,
            1.,
            100000,
        )
        .unwrap();
        let path = Curve {
            degree: 3,
            knots: vec![
                0., 0., 0., 0., 0.25, 0.25, 0.25, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 1., 1., 1., 1.,
            ],
            control_points: vec![
                vec![3., 0., 0.],
                vec![3., 1., 0.125],
                vec![1., 3., 0.125],
                vec![0., 3., 0.],
                vec![-1., 3., -0.125],
                vec![-3., 1., -0.125],
                vec![-3., 0., 0.],
                vec![-3., -1., 0.125],
                vec![-1., -3., 0.125],
                vec![0., -3., 0.],
                vec![1., -3., -0.125],
                vec![3., -1., -0.125],
                vec![3., 0., 0.],
            ],
            weights: vec![1.; 13],
            periodic: false,
        };
        let scale = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        (
            json!({"profiles":sections[0],"path":path,"scale":scale,"normal":[1,0,0],
            "maxDeviation":1,"maxCells":100000}),
            candidate.sides.unwrap(),
        )
    }
    #[test]
    fn all_retained_spatial_walls_need_complete_shared_source_budget() {
        let (source, sides) = fixture();
        let full = certify(&source, &sides, true).unwrap();
        assert_eq!(full["withinBudget"], json!(true), "{full}");
        assert!(full["errorUpper"].as_f64().unwrap() <= 1.);
        assert_eq!(full["walls"].as_array().unwrap().len(), 4);
        for budget in [0, 1] {
            let mut limited = source.clone();
            limited["maxCells"] = json!(budget);
            let denied = certify(&limited, &sides, true).unwrap();
            assert_eq!(denied["withinBudget"], json!(false));
            assert!(denied["errorUpper"].is_null());
            assert!(denied["cells"].as_u64().unwrap() <= budget);
        }
    }
    #[test]
    fn source_family_and_retained_join_mutations_cannot_reuse_the_proof() {
        let (mut source, mut sides) = fixture();
        source["orientationGuide"] = json!(true);
        assert!(certify(&source, &sides, true).is_err());
        let (mut gap, _) = fixture();
        let coordinate = gap["profiles"][0][1]["controlPoints"][0][0]
            .as_f64()
            .unwrap();
        gap["profiles"][0][1]["controlPoints"][0][0] = json!(coordinate.next_up());
        assert!(certify(&gap, &sides, true).is_err());
        let (source, _) = fixture();
        sides[0][4].control_points[0][0][0] = sides[0][4].control_points[0][0][0].next_up();
        assert!(certify(&source, &sides, true).is_err());
        assert_eq!(
            certify(&source, &sides, false).unwrap()["withinBudget"],
            json!(false)
        );
    }
}

#[cfg(test)]
pub(super) fn test_source() -> Value {
    tests::fixture().0
}
