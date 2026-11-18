use geometry_bridge::{dispatch, execute};
use value_codec::{Value, json};

fn request() -> Value {
    let a = json!({
        "degreeU": 1, "degreeV": 1,
        "knotsU": [2, 2, 5, 5], "knotsV": [1, 1, 3, 3],
        "controlPoints": [[[0, 0, 0], [0, 1, 0]], [[1, 0, 0], [1, 1, 0]]],
        "weights": [[1, 1], [1, 1]],
    });
    let u = [1., -2., 4., -8., 16.];
    let v = [1., -1., 1., -1., 1.];
    let points: Vec<_> = (0..5)
        .map(|i| {
            (0..5)
                .map(|j| [i as f64 / 4., j as f64 / 4., u[i] + v[j]])
                .collect::<Vec<_>>()
        })
        .collect();
    let b = json!({
        "degreeU": 4, "degreeV": 4,
        "knotsU": [-1, -1, -1, -1, -1, 1, 1, 1, 1, 1],
        "knotsV": [4, 4, 4, 4, 4, 8, 8, 8, 8, 8],
        "controlPoints": points, "weights": vec![vec![1; 5]; 5],
    });
    json!({"op": "brep_qualify_isolated_ss_contact", "a": a, "b": b})
}

#[test]
fn json_dispatch_preserves_exact_contact_coordinates_in_both_domains() {
    let reply: Value = value_codec::from_str(&execute(&request().to_string())).unwrap();
    assert_eq!(reply["ok"], true, "{reply}");
    let value = &reply["value"];
    assert_eq!(value["uvA"], json!(["3", "2"]));
    assert_eq!(value["uvB"], json!(["-1/3", "6"]));
    assert_eq!(value["centerA"], json!(["1/3", "1/2"]));
    assert_eq!(value["powers"], json!([4, 4]));
    assert_eq!(value["proof"], "definite-powers");
    assert_eq!(value["mixedCoefficient"], "0");
    assert_eq!(value["coversEntireSurfaces"], true);
}

#[test]
fn unproved_contact_has_a_distinct_error_and_accepts_no_client_claims() {
    let mut damaged = request();
    damaged["b"]["controlPoints"][2][2][2] = json!(1. / 1024.);
    assert_eq!(
        dispatch(damaged).unwrap_err().code,
        "BREP_SS_ISOLATED_CONTACT_UNPROVED",
    );
    let mut claimed = request();
    claimed["coversEntireSurfaces"] = json!(true);
    assert!(dispatch(claimed).is_err());
}

#[test]
fn json_dispatch_returns_exact_coordinates_for_the_hessian_certificate() {
    let mut input = request();
    let u = [1., -2., 4.];
    let v = [1., -1., 1.];
    let points: Vec<_> = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| [i as f64 / 2., j as f64 / 2., u[i] + v[j] + u[i] * v[j] / 8.])
                .collect::<Vec<_>>()
        })
        .collect();
    input["b"] = json!({
        "degreeU": 2, "degreeV": 2,
        "knotsU": [-1, -1, -1, 1, 1, 1],
        "knotsV": [4, 4, 4, 8, 8, 8],
        "controlPoints": points, "weights": vec![vec![1; 3]; 3],
    });
    let value = dispatch(input).unwrap();
    assert_eq!(value["proof"], "definite-hessian");
    assert_eq!(value["uvA"], json!(["3", "2"]));
    assert_eq!(value["uvB"], json!(["-1/3", "6"]));
    assert!(value.get("powers").is_none());
}
