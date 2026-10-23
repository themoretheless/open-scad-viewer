//! Read-only diagnostic on actual stored BREP surfaces. A chart result alone
//! is not world-edge ownership, individual injectivity or Solid admission.
use brep_core::Model;
fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    assert!(
        (3..=17).contains(&arguments.len()) && arguments.len() % 2 == 1,
        "Usage: model.json face_a face_b [face_c face_d ...], at most eight pairs"
    );
    let model: Model =
        value_codec::from_str(&std::fs::read_to_string(&arguments[0]).unwrap()).unwrap();
    model.validate().unwrap();
    for pair in arguments[1..].chunks_exact(2) {
        let faces = [
            pair[0].parse::<usize>().unwrap(),
            pair[1].parse::<usize>().unwrap(),
        ];
        println!("Faces: {faces:?}");
        assert!(faces[0] != faces[1] && faces.iter().all(|&f| f < model.faces.len()));
        for axes in [[0, 0], [0, 1], [1, 0], [1, 1]] {
            let charts = std::array::from_fn::<_, 2, _>(|i| {
                if axes[i] == 0 {
                    model.faces[faces[i]].surface.clone()
                } else {
                    nurbs_core::surface_edit::transpose(&model.faces[faces[i]].surface).unwrap()
                }
            });
            for ends in [[false, false], [false, true], [true, false], [true, true]] {
                let rows = std::array::from_fn::<_, 2, _>(|i| {
                    if ends[i] {
                        charts[i].control_points.len() - 1
                    } else {
                        0
                    }
                });
                if charts[0].control_points[rows[0]] != charts[1].control_points[rows[1]] {
                    continue;
                }
                let report =
                    nurbs_core::analysis::surface_injectivity::certify_shared_natural_boundary(
                        &charts[0], &charts[1], ends, 6,
                    )
                    .unwrap();
                println!("axes={axes:?} ends={ends:?} chart={report:?}");
            }
        }
        println!(
            "Owned boundary: {:?}",
            brep_core::shared_boundary::inspect_opposite_pair(&model, faces).unwrap()
        );
    }
}
