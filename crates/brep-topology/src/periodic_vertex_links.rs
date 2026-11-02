use super::*;
fn torus() -> Model<(), (), ()> {
    Model {
        vertices: vec![Vertex { point: [0.; 3] }],
        edges: (0..2)
            .map(|_| Edge {
                vertices: [0, 0],
                curve: (),
                degenerate: false,
            })
            .collect(),
        loops: vec![Loop {
            coedges: vec![
                Coedge::new(0, false, ()),
                Coedge::new(1, false, ()),
                Coedge::new(0, true, ()),
                Coedge::new(1, true, ()),
            ],
        }],
        faces: vec![Face {
            surface: (),
            outer: 0,
            holes: vec![],
        }],
        shells: vec![Shell {
            faces: vec![FaceUse {
                face: 0,
                reversed: false,
            }],
            closed: true,
        }],
        bodies: vec![Body {
            outer_shell: 0,
            inner_shells: vec![],
        }],
        tolerance_mm: 1e-7,
    }
}
#[test]
fn two_ends_of_closed_edges_form_one_valid_torus_vertex_link() {
    let m = torus();
    assert!(m.validate_topology().is_ok());
    let mut reverse = m.clone();
    reverse.shells[0].faces[0].reversed = true;
    assert!(reverse.validate_topology().is_ok());
}
#[test]
fn pinched_closed_edge_fans_and_excess_or_same_direction_uses_stay_invalid() {
    let mut pinched = torus();
    pinched.edges.extend(torus().edges);
    let mut hole = torus().loops.remove(0);
    for c in &mut hole.coedges {
        c.edge += 2;
    }
    pinched.loops.push(hole);
    pinched.faces[0].holes.push(1);
    let error = pinched.validate_topology().unwrap_err();
    assert!(
        error.message.contains("disconnected face fans"),
        "{error:?}"
    );
    let mut excess = torus();
    excess.loops[0].coedges.push(Coedge::new(0, false, ()));
    assert!(excess.validate_topology().is_err());
    let mut same = torus();
    same.loops[0].coedges[2].reversed = false;
    assert!(same.validate_topology().is_err());
}
