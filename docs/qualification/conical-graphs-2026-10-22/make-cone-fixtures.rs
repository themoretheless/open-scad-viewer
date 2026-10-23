//! Native fixture data builder. Production certification is a separate process.
use std::{fs, path::PathBuf};
fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("fixture directory"));
    for (bottom, top, name) in [
        (1., 0., "nonsmooth-rational-cone"),
        (0., 1., "inverted-rational-cone"),
    ] {
        let m = brep_core::frustum(bottom, top, 2.).unwrap();
        m.validate().unwrap();
        fs::write(
            dir.join(format!("{name}.json")),
            value_codec::to_string(&m).unwrap(),
        )
        .unwrap();
    }
    let mut m = brep_core::frustum(1., 0., 2.).unwrap();
    let shear = |p: &mut [f64]| {
        let old = p.to_vec();
        p[0] += p[2] / 8.;
        p[1] -= p[2] / 16.;
        assert_eq!(p[0] - p[2] / 8., old[0]);
        assert_eq!(p[1] + p[2] / 16., old[1]);
    };
    for v in &mut m.vertices {
        shear(&mut v.point);
    }
    for e in &mut m.edges {
        for p in &mut e.curve.control_points {
            shear(p);
        }
    }
    for f in &mut m.faces {
        for r in &mut f.surface.control_points {
            for p in r {
                shear(p);
            }
        }
    }
    m.rebuild_topology_ids();
    m.validate().unwrap();
    fs::write(
        dir.join("oblique-rational-cone.json"),
        value_codec::to_string(&m).unwrap(),
    )
    .unwrap();
}
