use brep_core::Model;
use nurbs_core::curve::Curve;
#[path = "full_periodic_cylinder.rs"]
mod cylinder;
pub fn source(scale: f64, inverted: bool) -> Model {
    let mut s = cylinder::source();
    for row in &mut s.faces[0].surface.control_points {
        row[1][0] *= scale;
        row[1][1] *= scale;
    }
    for p in &mut s.edges[1].curve.control_points {
        p[0] *= scale;
        p[1] *= scale;
    }
    s.vertices[1].point = [scale, 0., 2.];
    s.edges[2].curve.control_points[1] = s.vertices[1].point.to_vec();
    for row in &mut s.faces[2].surface.control_points {
        for p in row {
            p[0] *= scale;
            p[1] *= scale;
        }
    }
    if scale == 0. {
        s.edges[1].curve = Curve::from_polyline(vec![s.vertices[1].point.to_vec(); 2]).unwrap();
        s.edges[1].degenerate = true;
        s.faces.remove(2);
        s.loops.remove(2);
        s.shells[0].faces.pop();
    }
    if inverted {
        for row in &mut s.faces[0].surface.control_points {
            row.swap(0, 1);
        }
        for row in &mut s.faces[0].surface.weights {
            row.swap(0, 1);
        }
        s.loops[0].coedges.reverse();
        for c in &mut s.loops[0].coedges {
            c.pcurve = c.pcurve.reverse().unwrap();
            for p in &mut c.pcurve.control_points {
                p[1] = 1. - p[1];
            }
            c.reversed = !c.reversed;
        }
        s.shells[0].faces[0].reversed = true;
    }
    s.rebuild_topology_ids();
    s.validate().unwrap();
    s
}
