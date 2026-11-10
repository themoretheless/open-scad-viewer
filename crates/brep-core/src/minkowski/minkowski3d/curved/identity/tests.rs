use super::*;
use value_codec::Serialize;
fn same<T: Put + Serialize>(v: &T) {
    assert_eq!(
        encode(v).unwrap(),
        value_codec::encode_binary(&v.to_value()).unwrap()
    )
}
#[test]
fn geometry_and_full_naming_records_keep_legacy_bytes() {
    let mut m = crate::cuboid([2., 3., 4.], [7., 11., 13.]).unwrap();
    same(&m);
    same(&Source::Brep(m.clone()));
    m.vertices[0].point[0] = -0.;
    let id = m.1.vertices[0];
    m.1.local_tolerances_mm.insert(id, 1e-7);
    m.1.sewed_edges.insert(m.1.edges[0]);
    m.1.lineage.push(crate::TopologyLineageRecord {
        operation: "сшивка 🛠".into(),
        entity_kind: "edge".into(),
        parents: vec![id],
        children: vec![],
    });
    m.1.change_set.changes.push(crate::TopologyChange {
        kind: crate::ChangeKind::Split,
        topo_kind: crate::TopoKind::Edge,
        parents: vec![m.1.edges[0]],
        children: vec![],
        provenance: crate::ChangeProvenance {
            operation: "edit".into(),
            operand: Some("левый".into()),
            occurrence: "a/b".into(),
        },
        role: "seam".into(),
        anchor: Some("anchor".into()),
    });
    m.edges[0].degenerate = true;
    m.loops[0].coedges[0] = m.loops[0].coedges[0]
        .clone()
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: [1., 0.],
            pcurve_parameter: [0., 1.],
            periodic_lift: [[-2, 1], [3, -4]],
        })
        .unwrap();
    same(&m);
    same(&Source::Brep(m.clone()));
    m.tolerance_mm = f64::NAN;
    same(&m); // Legacy encoder omits invalid context and emits Null.
}
#[test]
fn rational_periodic_surfaces_and_wire_keys_keep_legacy_bytes() {
    for m in [
        crate::cylinder(2., 3.).unwrap(),
        crate::sphere(2.).unwrap(),
        crate::torus(3., 1.).unwrap(),
    ] {
        same(&m);
        same(&Source::Brep(m.clone()));
        for f in &m.faces {
            same(&f.surface);
            assert_eq!(
                surface(&f.surface).unwrap(),
                value_codec::encode_binary(&f.surface.to_value()).unwrap()
            )
        }
        let curves = m.edges.iter().map(|e| e.curve.clone()).collect::<Vec<_>>();
        same(&curves);
        assert_eq!(
            wire(&curves).unwrap(),
            value_codec::encode_binary(&curves.to_value()).unwrap()
        );
    }
    same(&Source::Mesh(SourceMesh {
        positions: vec![-0., 1., 2., 3.],
        indices: vec![0, 1, 2],
    }));
    same(&vec![i32::MIN, -1, 0, 1, i32::MAX]);
    same(&vec![
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        -0.,
        f64::INFINITY,
    ]);
}
struct Nested(usize);
impl Put for Nested {
    fn put(&self, e: &mut Encoder, d: usize) -> Result<()> {
        if self.0 == 0 {
            false.put(e, d)
        } else {
            e.tag(d, 5)?;
            e.length(1)?;
            Nested(self.0 - 1).put(e, d + 1)
        }
    }
}
#[test]
fn original_binary_nesting_and_size_bounds_are_not_raised() {
    assert!(encode(&Nested(128)).is_ok());
    assert!(encode(&Nested(129)).is_err());
    let large = "x".repeat(32 * 1024 * 1024);
    assert!(encode(&large).is_err());
}
