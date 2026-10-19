use geometry_bridge::dispatch;
use value_codec::{Value, json};
#[path = "support/fibered_body.rs"]
mod fibers;
#[path = "support/implicit_sphere.rs"]
mod sphere;

fn cases() -> Vec<(&'static str, &'static str, Value, &'static str)> {
    let mut g1 = fibers::three_piece_request();
    g1["source"]["motion"][1]["controls"][2][0][0] = json!(1.625);
    g1["limits"]["order"] = json!(1);
    vec![
        (
            "unified-sphere-poles",
            "implicit_sphere",
            sphere::request(2., [0.; 3]),
            "G2",
        ),
        (
            "unified-sphere-translated",
            "implicit_sphere",
            sphere::request(2., [8., -4., 6.]),
            "G2",
        ),
        (
            "unified-sphere-nonbinary",
            "implicit_sphere",
            sphere::request(3_f64.next_up(), [0.; 3]),
            "G2",
        ),
        ("unified-fiber-caps", "fibered", fibers::request(), "G2"),
        (
            "unified-fiber-oblique",
            "fibered",
            fibers::oblique_request(false),
            "G2",
        ),
        (
            "unified-fiber-descending",
            "fibered",
            fibers::oblique_request(true),
            "G2",
        ),
        (
            "unified-fiber-multipiece-g2",
            "fibered",
            fibers::three_piece_request(),
            "G2",
        ),
        ("unified-fiber-multipiece-g1", "fibered", g1, "G1"),
        (
            "unified-fiber-cavity-c0",
            "fibered",
            fibers::hole_request(0),
            "C0",
        ),
    ]
}
#[test]
fn original_source_strata_tessellate_with_closed_manifold_seams() {
    for (name, _, input, _) in cases() {
        let built = dispatch(input).unwrap();
        for segments in [2, 4, 8] {
            let mesh = dispatch(
                json!({"op":"brep_nurbs_tessellate", "model":built["model"], "segments":segments}),
            )
            .unwrap_or_else(|e| panic!("{name}, segments {segments}: {e:?}"));
            assert_eq!(mesh["report"]["closed"], true, "{name}: {mesh}");
            for key in [
                "nonManifoldEdges",
                "boundaryEdges",
                "orientationConflicts",
                "degenerateTriangles",
            ] {
                assert_eq!(mesh["report"][key], 0, "{name}/{segments}/{key}");
            }
        }
    }
}
