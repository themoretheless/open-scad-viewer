use super::*;
use value_codec::json;

fn request(limit: f64) -> Value {
    json!({"options":{"pattern":"grid","axis":"x","cell":20.,"rib":1.35,
        "rim":2.,"bottom":0.1,"top":0.7},
        "settings":{"nozzle":0.6,"layer":0.25,"lines":4,"skinLayers":4,
        "maxBridge":limit,"openTop":true},"limitOpening":true})
}

#[test]
fn opening_fit_is_bounded_and_idempotent() {
    let v = request(5.);
    let fitted = print_fit(v.clone()).unwrap();
    let rib = fitted["rib"].as_f64().unwrap();
    let cell = fitted["cell"].as_f64().unwrap();
    assert_eq!(rib, 2.7);
    assert!(cell - rib <= 5.);
    assert!(cell >= 2. * rib + fitted["lineWidth"].as_f64().unwrap());
    let mut again = v;
    again["options"] = fitted.clone();
    assert_eq!(print_fit(again).unwrap(), fitted);
}

#[test]
fn opening_fit_refuses_impossible_and_malformed_constraints() {
    assert!(
        print_fit(request(1.))
            .unwrap_err()
            .message
            .contains("incompatible")
    );
    let mut v = request(5.);
    v["limitOpening"] = json!("true");
    assert!(print_fit(v).unwrap_err().message.contains("boolean"));
    for nozzle in [1e-20, f64::MAX] {
        let mut v = request(5.);
        v["settings"]["nozzle"] = json!(nozzle);
        v["settings"]["layer"] = json!(nozzle);
        assert!(print_fit(v).unwrap_err().message.contains("numeric range"));
    }
}

#[test]
fn opening_fit_respects_representable_bounds() {
    for nozzle in [0.25, 0.4, 0.6, 0.8] {
        for lines in 1..=8 {
            for limit in [2., 3., 5., 10.] {
                let mut v = request(limit);
                v["settings"]["nozzle"] = json!(nozzle);
                v["settings"]["layer"] = json!(0.2);
                v["settings"]["lines"] = json!(lines);
                let mut unlimited = v.clone();
                unlimited["limitOpening"] = json!(false);
                let base = print_fit(unlimited).unwrap();
                let rib = base["rib"].as_f64().unwrap();
                let minimum = 2. * rib + base["lineWidth"].as_f64().unwrap();
                match print_fit(v) {
                    Ok(fit) => {
                        let cell = fit["cell"].as_f64().unwrap();
                        assert!(cell >= minimum && cell - rib <= limit);
                        assert_eq!(fit["rib"], base["rib"]);
                    }
                    Err(_) => assert!(minimum - rib > limit),
                }
            }
        }
    }
}
