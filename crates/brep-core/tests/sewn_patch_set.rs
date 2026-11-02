#[path = "../examples/support/region_network_fixture.rs"]
mod fixture;
use brep_core::region_replacement as r;
#[test]
fn simultaneous_patches_create_a_new_shared_interface_in_a_closed_g2_body() {
    let mut f = fixture::build();
    // Unrelated document geometry is represented by a valid second body;
    // isolated unreferenced points/edges are not valid B-rep input.
    let other = brep_core::cuboid([20.; 3], [21.; 3]).unwrap();
    let (v0, e0, w0, f0, s0) = (
        f.source.vertices.len(),
        f.source.edges.len(),
        f.source.loops.len(),
        f.source.faces.len(),
        f.source.shells.len(),
    );
    f.source.vertices.extend(other.vertices.iter().cloned());
    f.source
        .edges
        .extend(other.edges.iter().cloned().map(|mut e| {
            e.vertices = e.vertices.map(|v| v + v0);
            e
        }));
    f.source
        .loops
        .extend(other.loops.iter().cloned().map(|mut l| {
            for c in &mut l.coedges {
                c.edge += e0;
            }
            l
        }));
    f.source
        .faces
        .extend(other.faces.iter().cloned().map(|mut f| {
            f.outer += w0;
            for w in &mut f.holes {
                *w += w0;
            }
            f
        }));
    f.source
        .shells
        .extend(other.shells.iter().cloned().map(|mut s| {
            for f in &mut s.faces {
                f.face += f0;
            }
            s
        }));
    f.source
        .bodies
        .extend(other.bodies.iter().cloned().map(|mut b| {
            b.outer_shell += s0;
            for s in &mut b.inner_shells {
                *s += s0;
            }
            b
        }));
    f.source.rebuild_topology_ids();
    f.source.validate().unwrap();
    let before = f.source.clone();
    let selected = f.selected.iter().flatten().copied().collect::<Vec<_>>();
    let old = f.cut_edges[0];
    let edges = [f.source.edges[old].clone()];
    let mut loops = f.loops.clone();
    for c in loops.iter_mut().flatten().flatten() {
        if c.edge == old {
            c.edge = f.source.edges.len();
        }
    }
    let patches = [0, 1].map(|i| r::SewPatch {
        surface: &f.chart,
        loops: &loops[i],
        reversed: f.reversed[i],
    });
    let limits = r::ContactQualificationLimits::default();
    let out = r::sew_g2_patch_set(
        &f.source,
        &selected,
        &[],
        &edges,
        &patches,
        "new-g2-diagonal",
        1e-8,
        limits,
    )
    .unwrap();
    assert_eq!(f.source, before);
    assert_eq!(out.model().bodies.len(), 2);
    for i in f0..f.source.faces.len() {
        let kept = out.source_faces()[i].unwrap();
        assert_eq!(out.model().faces[kept].surface, f.source.faces[i].surface);
        assert_eq!(out.model().1.faces[kept], f.source.1.faces[i]);
    }
    for i in v0..f.source.vertices.len() {
        let kept = out.source_vertices()[i].unwrap();
        assert_eq!(out.model().vertices[kept], f.source.vertices[i]);
    }
    assert_eq!(out.patch_faces().len(), 2);
    assert!(out.source_edges()[old].is_none());
    assert_eq!(out.new_edges().len(), 1);
    let seam = out.new_edges()[0];
    assert_eq!(
        out.model()
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == seam)
            .count(),
        2
    );
    let actual = out.seams().seams.iter().find(|p| p.edge == seam).unwrap();
    assert!(actual.faces.iter().all(|f| out.patch_faces().contains(f)));
    assert!(out.seams().complete && out.seams().within_tolerance);
    assert_eq!(
        out.closure().closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(
        out.fields()
            .iter()
            .all(|p| p.absolute_curvature_upper.unwrap() <= 10.)
    );
    for (i, mapped) in out.source_edges().iter().enumerate() {
        if let Some(j) = mapped {
            assert_eq!(f.source.1.edges[i], out.model().1.edges[*j]);
        }
    }
    let (c, e, d, s) = out.work();
    assert!(
        c <= limits.cells
            && e <= limits.exact_work
            && d <= limits.domain_cells
            && s <= limits.shape_cells
    );
    #[cfg(feature = "codec")]
    {
        let loaded: brep_core::Model =
            value_codec::from_str(&value_codec::to_string(out.model()).unwrap()).unwrap();
        assert_eq!(&loaded, out.model());
        assert_eq!(
            brep_core::watertight::inspect_with_exact(&loaded, 100_000, 1_000_000)
                .unwrap()
                .closure
                .status,
            brep_core::watertight::Closure::WithinTolerance
        );
    }
    eprintln!(
        "new_interface faces={} edges={} vertices={} g2={} curvature_upper={} work={:?}",
        out.model().faces.len(),
        out.model().edges.len(),
        out.model().vertices.len(),
        out.seams().seams.len(),
        out.fields()
            .iter()
            .map(|p| p.absolute_curvature_upper.unwrap())
            .fold(0., f64::max),
        out.work()
    );
}
#[test]
fn missing_or_reversed_interfaces_and_unused_roles_refuse_atomically() {
    let f = fixture::build();
    let before = f.source.clone();
    let selected = f.selected.iter().flatten().copied().collect::<Vec<_>>();
    let old = f.cut_edges[0];
    let edges = [f.source.edges[old].clone()];
    let mut loops = f.loops.clone();
    for c in loops.iter_mut().flatten().flatten() {
        if c.edge == old {
            c.edge = f.source.edges.len();
        }
    }
    let l = r::ContactQualificationLimits::default();
    let call = |loops: &[Vec<Vec<brep_core::Coedge>>; 2],
                edges: &[brep_core::Edge],
                vertices: &[brep_core::Vertex],
                limit| {
        let p = [0, 1].map(|i| r::SewPatch {
            surface: &f.chart,
            loops: &loops[i],
            reversed: f.reversed[i],
        });
        r::sew_g2_patch_set(
            &f.source, &selected, vertices, edges, &p, "refusal", 1e-8, limit,
        )
        .is_err()
    };
    assert!(call(
        &loops,
        &edges,
        &[],
        r::ContactQualificationLimits { cells: 8, ..l }
    ));
    assert!(call(
        &loops,
        &edges,
        &[brep_core::Vertex { point: [9.; 3] }],
        l
    ));
    let extra = [edges[0].clone(), edges[0].clone()];
    assert!(call(&loops, &extra, &[], l));
    let co = loops[1]
        .iter_mut()
        .flatten()
        .find(|c| c.edge == f.source.edges.len())
        .unwrap();
    co.reversed = !co.reversed;
    assert!(call(&loops, &edges, &[], l));
    assert_eq!(f.source, before);
}

#[test]
fn six_patch_node_introduces_one_shared_vertex_and_six_new_g2_interfaces() {
    use nurbs_core::{curve::Curve, curve_surface_agreement};
    let f = fixture::build();
    let selected = f.selected.iter().flatten().copied().collect::<Vec<_>>();
    let old = f.cut_edges[0];
    let boundary = f.loops[0][0].iter().find(|c| c.edge != old).unwrap().edge;
    let d = f.source.edges[boundary].curve.domain();
    let divided = brep_core::feature_ops::edge_partition::split(
        &f.source,
        boundary,
        &[d[0] + (d[1] - d[0]) * 0.25, d[0] + (d[1] - d[0]) * 0.75],
        "six-node-source-rim",
        Default::default(),
    )
    .unwrap();
    let source = divided.model();
    let before = source.clone();
    let mut incidence = std::collections::BTreeMap::<usize, Vec<brep_core::Coedge>>::new();
    for &face in &selected {
        for c in &source.loops[source.faces[face].outer].coedges {
            incidence.entry(c.edge).or_default().push(c.clone());
        }
    }
    let mut remaining = incidence
        .values()
        .filter(|v| v.len() == 1)
        .map(|v| v[0].clone())
        .collect::<Vec<_>>();
    let mut rim = vec![remaining.remove(0)];
    while !remaining.is_empty() {
        let c = rim.last().unwrap();
        let end = source.edges[c.edge].vertices[usize::from(!c.reversed)];
        let i = remaining
            .iter()
            .position(|c| source.edges[c.edge].vertices[usize::from(c.reversed)] == end)
            .unwrap();
        rim.push(remaining.remove(i));
    }
    assert_eq!(rim.len(), 6);
    let uv = [0.5, 0.5];
    let p = f.chart.evaluate(uv[0], uv[1]).unwrap().point;
    let vertices = [brep_core::Vertex {
        point: p.clone().try_into().unwrap(),
    }];
    let center = source.vertices.len();
    let mut edges = Vec::new();
    let mut pcs = Vec::new();
    for c in &rim {
        let start = source.edges[c.edge].vertices[usize::from(c.reversed)];
        let pc =
            Curve::from_polyline(vec![c.pcurve.control_points[0].clone(), uv.to_vec()]).unwrap();
        let curve = curve_surface_agreement::lift(
            &pc,
            &f.chart,
            Some([source.vertices[start].point, p.clone().try_into().unwrap()]),
            1e-7,
            100_000,
        )
        .unwrap()
        .curve
        .unwrap();
        edges.push(brep_core::Edge {
            vertices: [start, center],
            curve,
            degenerate: false,
        });
        pcs.push(pc);
    }
    let loops = (0..6)
        .map(|i| {
            vec![vec![
                rim[i].clone(),
                brep_core::Coedge::new(
                    source.edges.len() + (i + 1) % 6,
                    false,
                    pcs[(i + 1) % 6].clone(),
                ),
                brep_core::Coedge::new(source.edges.len() + i, true, pcs[i].reverse().unwrap()),
            ]]
        })
        .collect::<Vec<_>>();
    let patches = loops
        .iter()
        .map(|loops| r::SewPatch {
            surface: &f.chart,
            loops,
            reversed: f.reversed[0],
        })
        .collect::<Vec<_>>();
    let out = r::sew_g2_patch_set(
        source,
        &selected,
        &vertices,
        &edges,
        &patches,
        "six-valent-g2-node",
        1e-8,
        Default::default(),
    )
    .unwrap();
    assert_eq!(source, &before);
    assert_eq!(out.new_vertices().len(), 1);
    assert_eq!(out.new_edges().len(), 6);
    assert_eq!(out.patch_faces().len(), 6);
    assert_eq!(
        out.model()
            .edges
            .iter()
            .filter(|e| e.vertices.contains(&out.new_vertices()[0]))
            .count(),
        6
    );
    assert!(out.seams().complete && out.seams().within_tolerance);
    assert_eq!(
        out.closure().closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(out.source_edges()[old].is_none());
    eprintln!(
        "six_patch_node faces={} edges={} vertices={} seams={} curvature_upper={} work={:?}",
        out.model().faces.len(),
        out.model().edges.len(),
        out.model().vertices.len(),
        out.seams().seams.len(),
        out.fields()
            .iter()
            .map(|p| p.absolute_curvature_upper.unwrap())
            .fold(0., f64::max),
        out.work()
    );
}

#[test]
fn closed_positionally_matching_network_with_a_new_g2_crease_is_refused() {
    let f = fixture::build();
    let before = f.source.clone();
    let old = f.cut_edges[0];
    let selected = f.selected.iter().flatten().copied().collect::<Vec<_>>();
    let edges = [f.source.edges[old].clone()];
    let mut loops = f.loops.clone();
    for c in loops.iter_mut().flatten().flatten() {
        if c.edge == old {
            c.edge = f.source.edges.len();
        }
    }
    // The crease polynomial is authored in a quintic basis; production charts
    // may now preserve their exact lower degree.
    let elevated = nurbs_core::surface_edit::elevate(&f.chart, nurbs_core::surface::Axis::U, 5).unwrap();
    let mut kink = nurbs_core::surface_edit::elevate(&elevated, nurbs_core::surface::Axis::V, 5).unwrap();
    assert_eq!((kink.degree_u, kink.degree_v), (5, 5));
    let choose = |n: usize, k: usize| {
        if k > n {
            0.
        } else {
            (0..k).fold(1., |v, i| v * (n - i) as f64 / (i + 1) as f64)
        }
    };
    // u*v*(u-v)*(1-u)*(1-v) is zero on every outer rim and
    // the shared diagonal, but changes the transverse boundary jets.
    let terms = [
        (2, 1, 1.),
        (3, 1, -1.),
        (3, 2, 1.),
        (1, 2, -1.),
        (1, 3, 1.),
        (2, 3, -1.),
    ];
    for i in 0..6 {
        for j in 0..6 {
            let delta = terms
                .iter()
                .map(|&(a, b, sign)| {
                    sign * choose(i, a) / choose(5, a) * choose(j, b) / choose(5, b)
                })
                .sum::<f64>();
            kink.control_points[i][j][2] += 0.1 * delta;
        }
    }
    let patches = [
        r::SewPatch {
            surface: &f.chart,
            loops: &loops[0],
            reversed: f.reversed[0],
        },
        r::SewPatch {
            surface: &kink,
            loops: &loops[1],
            reversed: f.reversed[1],
        },
    ];
    let err = match r::sew_g2_patch_set(
        &f.source,
        &selected,
        &[],
        &edges,
        &patches,
        "crease-refusal",
        1e-8,
        Default::default(),
    ) {
        Ok(_) => panic!("C0 cannot authorize a G2 crease"),
        Err(e) => e,
    };
    assert!(err.message.contains("G2"), "{err:?}");
    assert_eq!(f.source, before);
}
