use brep_core::{
    Model,
    feature_ops::edge_partition::{self, Limits, seam::compact},
};
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn source() -> Model {
    edge_partition::split(
        &fixture::source(),
        0,
        &[0.5, 1.5, 2.5],
        "split",
        Limits::default(),
    )
    .unwrap()
    .model()
    .clone()
}
#[test]
fn an_actual_four_edge_cycle_becomes_one_admitted_closed_curve_and_one_vertex() {
    let mut s = source();
    let local_vertex = s.1.vertices[2];
    s.1.local_tolerances_mm.insert(local_vertex, 1e-8);
    let saved = s.clone();
    let r = compact::compact(&s, 1, "compact", Limits::default()).unwrap();
    let m = r.model();
    assert_eq!(
        m.1.local_tolerances_mm.get(&m.1.edges[r.edge()]),
        Some(&1e-8)
    );
    assert_eq!(m.edges.len(), 1);
    assert_eq!(m.vertices.len(), 1);
    assert_eq!(m.edges[r.edge()].vertices, [r.seam_vertex(); 2]);
    assert_eq!(m.1.faces, s.1.faces);
    assert_eq!(m.1.bodies, s.1.bodies);
    assert_eq!(r.segments().len(), 4);
    assert!(
        m.1.change_set
            .changes
            .iter()
            .any(|c| c.kind == brep_core::ChangeKind::Merge
                && c.parents.len() == 4
                && c.children == [m.1.edges[r.edge()]])
    );
    assert_eq!(
        m.1.change_set
            .changes
            .iter()
            .filter(|c| c.kind == brep_core::ChangeKind::Deleted)
            .count(),
        3
    );
    assert!(m.persistent_naming_complete());
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert!((r.evidence().numerical_volume_mm3 - std::f64::consts::PI).abs() < 1e-6);
    let next =
        edge_partition::split(m, r.edge(), &[0.375, 2.25], "again", Limits::default()).unwrap();
    assert!(next.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
}
#[test]
fn mixed_edge_directions_and_codec_transport_retain_the_actual_closed_boundary() {
    let mut s = source();
    s.edges[1].curve = s.edges[1].curve.reverse().unwrap();
    s.edges[1].vertices.swap(0, 1);
    for l in &mut s.loops {
        for c in &mut l.coedges {
            if c.edge == 1 {
                c.reversed = !c.reversed;
            }
        }
    }
    let r = compact::compact(&s, 1, "mixed", Limits::default()).unwrap();
    assert!(r.source_cycle().iter().any(|c| c.reversed));
    assert!(r.model().loops.iter().all(|l| l.coedges[0].reversed));
    #[cfg(feature = "codec")]
    {
        let loaded = Model::from_value(r.model().to_value()).unwrap();
        assert_eq!(&loaded, r.model());
    }
    assert!(r.reverify().unwrap().trim_regions_valid);
}
#[test]
fn a_single_authored_clock_is_retained_and_exhausted_work_cannot_publish() {
    let mut s = source();
    let c = &mut s.loops[0].coedges[0];
    let d = c.pcurve.domain();
    *c = c
        .clone()
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: d,
            pcurve_parameter: d,
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
    let saved = s.clone();
    let compacted = compact::compact(&s, 1, "clock", Limits::default()).unwrap();
    assert_eq!(compacted.relations().len(), 1);
    assert!(compacted.model().loops[0].coedges[0].trim().is_some());
    assert!(compacted.reverify().unwrap().boundary_complete);
    assert_eq!(s, saved);
    let s = source();
    let saved = s.clone();
    assert!(
        compact::compact(
            &s,
            1,
            "tiny",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(s, saved);
}
#[test]
fn unrelated_body_geometry_and_authored_coedges_survive_index_compaction() {
    let mut s = source();
    let mut other = fixture::source();
    for v in &mut other.vertices {
        v.point[0] += 6.;
    }
    for e in &mut other.edges {
        for p in &mut e.curve.control_points {
            p[0] += 6.;
        }
    }
    for f in &mut other.faces {
        for row in &mut f.surface.control_points {
            for p in row {
                p[0] += 6.;
            }
        }
    }
    for l in &mut other.loops {
        let c = &mut l.coedges[0];
        *c = c
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [1., 3.],
                pcurve_parameter: [1., 3.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let (v, e, l, f, sh) = (
        s.vertices.len(),
        s.edges.len(),
        s.loops.len(),
        s.faces.len(),
        s.shells.len(),
    );
    for edge in &mut other.edges {
        edge.vertices = edge.vertices.map(|x| x + v);
    }
    for wire in &mut other.loops {
        for c in &mut wire.coedges {
            c.edge += e;
        }
    }
    for face in &mut other.faces {
        face.outer += l;
        for h in &mut face.holes {
            *h += l;
        }
    }
    for shell in &mut other.shells {
        for u in &mut shell.faces {
            u.face += f;
        }
    }
    for body in &mut other.bodies {
        body.outer_shell += sh;
        for h in &mut body.inner_shells {
            *h += sh;
        }
    }
    s.0.vertices.extend(other.0.vertices);
    s.0.edges.extend(other.0.edges);
    s.0.loops.extend(other.0.loops);
    s.0.faces.extend(other.0.faces);
    s.0.shells.extend(other.0.shells);
    s.0.bodies.extend(other.0.bodies);
    s.rebuild_topology_ids();
    let saved = s.clone();
    let r = compact::compact(&s, 1, "two-bodies", Limits::default()).unwrap();
    assert_eq!(r.model().edges.len(), 2);
    assert_eq!(r.model().vertices.len(), 2);
    assert_eq!(r.model().edges[0].curve, s.edges[e].curve);
    assert_eq!(
        r.model().loops[l].coedges[0].trim(),
        s.loops[l].coedges[0].trim()
    );
    assert_eq!(r.model().1.bodies, s.1.bodies);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(s, saved);
}
