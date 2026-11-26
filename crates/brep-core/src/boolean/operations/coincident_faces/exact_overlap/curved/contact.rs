use super::*;
use crate::model::planar_pcurve_regions::{CurveCuts, trace_cycles};

pub(super) fn intersection(
    wires: [&[Vec<Curve>]; 2],
    cuts: [Vec<CurveCuts>; 2],
    chart: &AffineChart,
    surface: &crate::Surface,
    reversed: bool,
    tolerance_mm: f64,
    max_nesting_pairs: usize,
) -> Result<Vec<Model>> {
    let cycles = trace_cycles(wires, cuts, PlanarOperation::Intersection)?;
    let regions = group_cycles(&cycles, max_nesting_pairs)?;
    regions
        .into_iter()
        .map(|region| {
            patch(
                region.into_iter().map(|i| cycles[i].as_slice()),
                chart,
                surface,
                reversed,
                tolerance_mm,
            )
        })
        .collect()
}

fn group_cycles(cycles: &[Vec<Curve>], budget: usize) -> Result<Vec<Vec<usize>>> {
    let n = cycles.len();
    if n.saturating_mul(n.saturating_sub(1)) > budget {
        return Err(crate::Error::new(
            "BREP_RESOURCE_LIMIT",
            "Curved contact exceeded its loop-nesting budget",
        ));
    }
    let mut containers = vec![Vec::new(); n];
    for (i, cycle) in cycles.iter().enumerate() {
        let p = &cycle[0].control_points[0];
        for (j, candidate) in cycles.iter().enumerate() {
            if i != j
                && certify_point_winding(std::slice::from_ref(candidate), [p[0], p[1]])?.windings[0]
                    != 0
            {
                containers[i].push(j);
            }
        }
    }
    let mut regions = BTreeMap::new();
    for (i, parents) in containers.iter().enumerate() {
        if parents.len() % 2 == 0 {
            regions.insert(i, vec![i]);
        }
    }
    for (i, parents) in containers.iter().enumerate() {
        if parents.len() % 2 == 1 {
            let mut nearest = parents
                .iter()
                .filter(|&&p| containers[p].len() + 1 == parents.len());
            let Some(&owner) = nearest.next().filter(|_| nearest.next().is_none()) else {
                return Err(unsupported("Curved contact has inconsistent nesting"));
            };
            regions
                .get_mut(&owner)
                .ok_or_else(|| unsupported("Curved contact hole has no outer cycle"))?
                .push(i);
        }
    }
    Ok(regions.into_values().collect())
}

fn patch<'a>(
    cycles: impl Iterator<Item = &'a [Curve]>,
    chart: &AffineChart,
    surface: &crate::Surface,
    reversed: bool,
    tolerance_mm: f64,
) -> Result<Model> {
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut loops = Vec::new();
    for cycle in cycles {
        let start = vertices.len();
        let mut coedges = Vec::new();
        for (i, uv) in cycle.iter().enumerate() {
            let mut world = uv.clone();
            for p in &mut world.control_points {
                let uv = [0, 1].map(|axis| Q::from_float(p[axis]).unwrap());
                let [x, y, z] = chart.lift(&uv);
                *p = vec![representable(&x)?, representable(&y)?, representable(&z)?];
            }
            let p = &world.control_points[0];
            vertices.push(Vertex {
                point: [p[0], p[1], p[2]],
            });
            coedges.push(Coedge::new(edges.len(), false, uv.clone()));
            edges.push(Edge {
                degenerate: false,
                vertices: [start + i, start + (i + 1) % cycle.len()],
                curve: world,
            });
        }
        loops.push(Loop { coedges });
    }
    let mut patch = Model(
        brep_topology::Model {
            vertices,
            edges,

            faces: vec![Face {
                surface: surface.clone(),
                outer: 0,
                holes: (1..loops.len()).collect(),
            }],
            loops,
            shells: vec![Shell {
                faces: vec![FaceUse { face: 0, reversed }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm,
        },
        TopologyIds::default(),
    );
    patch.rebuild_topology_ids();
    patch.validate()?;
    Ok(patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_cubic_contacts_build_one_exact_trim_cycle() {
        let box_model = crate::cuboid([0.; 3], [16., 16., 1.]).unwrap();
        let support = source_face(&box_model, 1, 100).unwrap();
        let surface = &box_model.faces[1].surface;
        let a = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: [14., 53., 11., 50.]
                .into_iter()
                .enumerate()
                .map(|(i, y)| vec![(1. + 3. * i as f64) / 16., y / 64.])
                .collect(),
            weights: vec![1.; 4],
            periodic: false,
        };
        let polygon = |points: &[[f64; 2]]| {
            edges(points)
                .map(|(p, q)| crate::line(p.to_vec(), q.to_vec()))
                .collect::<Vec<_>>()
        };
        let mut first = polygon(&[
            [1. / 16., 14. / 64.],
            [10. / 16., 50. / 64.],
            [10. / 16., 0.875],
            [1. / 16., 0.875],
        ]);
        first[0] = a;
        let second = polygon(&[[0., 0.5], [0.75, 0.5], [0.75, 1.], [0., 1.]]);
        let make_patch = |wire: &[Curve]| {
            patch(std::iter::once(wire), &support.chart, surface, false, 1e-7).unwrap()
        };
        let first = make_patch(&first);
        let second = make_patch(&second);
        let patches =
            strict_intersection([&first, &second], [0, 0], [&support, &support], 100).unwrap();
        assert_eq!(patches.len(), 1);
        let patch = &patches[0];
        assert_eq!(patch.edges.len(), 7);
        assert_eq!(
            patch.edges.iter().filter(|e| e.curve.degree == 3).count(),
            2
        );
        let mut links: Vec<_> = patch
            .edges
            .iter()
            .filter(|e| {
                e.curve.degree == 1 && e.vertices.iter().all(|&v| patch.vertices[v].point[1] == 8.)
            })
            .map(|e| e.vertices.map(|v| patch.vertices[v].point[0]))
            .collect();
        links.sort_by(|a, b| a[0].total_cmp(&b[0]));
        assert_eq!(links, [[1., 4.], [5.5, 7.]]);
        let agreement = crate::boundary_agreement::verify_exact(patch, 100_000).unwrap();
        assert!(agreement.all_equal && agreement.all_joins_exact);
    }

    #[test]
    fn nondyadic_quadratic_contacts_build_an_exact_brep_loop() {
        let box_model = crate::cuboid([0.; 3], [8., 8., 1.]).unwrap();
        let support = source_face(&box_model, 1, 100).unwrap();
        let surface = &box_model.faces[1].surface;
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.125, 0.75], vec![0.3125, 0.1875], vec![0.5, 0.75]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let polygon = |points: &[[f64; 2]]| {
            edges(points)
                .map(|(p, q)| crate::line(p.to_vec(), q.to_vec()))
                .collect::<Vec<_>>()
        };
        let mut first = polygon(&[[0.125, 0.75], [0.5, 0.75], [0.5, 0.875], [0.125, 0.875]]);
        first[0] = a.insert(0.5, 1).unwrap();
        assert!(exact_bezier_span_partition(&first[0]).unwrap().is_none());
        let second = polygon(&[[0.0625, 0.5], [0.8125, 0.5], [0.8125, 1.], [0.0625, 1.]]);
        let first = patch(
            std::iter::once(first.as_slice()),
            &support.chart,
            surface,
            false,
            1e-7,
        )
        .unwrap();
        let mut second = patch(
            std::iter::once(second.as_slice()),
            &support.chart,
            surface,
            false,
            1e-7,
        )
        .unwrap();
        second.loops[0].coedges[0] = Coedge::new(
            0,
            false,
            crate::line(vec![1., 0.5], vec![0., 0.5])
                .insert(0.5, 1)
                .unwrap(),
        )
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: [0., 1.],
            pcurve_parameter: [0.9375, 0.1875],
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
        second.rebuild_topology_ids();
        let agreement = crate::boundary_agreement::verify_exact(&second, 100_000).unwrap();
        assert!(agreement.all_equal && agreement.all_joins_exact);
        let models = [&first, &second];
        let patches = strict_intersection(models, [0, 0], [&support, &support], 100).unwrap();
        assert_eq!(patches.len(), 1);
        let patch = &patches[0];
        assert_eq!(patch.edges.len(), 6);
        assert_eq!(
            patch.edges.iter().filter(|e| e.curve.degree == 2).count(),
            2
        );
        let bridge = patch
            .edges
            .iter()
            .find(|e| e.vertices.iter().all(|&v| patch.vertices[v].point[1] == 4.))
            .unwrap();
        assert_eq!(
            bridge.vertices.map(|i| patch.vertices[i].point),
            [[2., 4., 1.], [3., 4., 1.]]
        );
        let agreement = crate::boundary_agreement::verify_exact(patch, 100_000).unwrap();
        assert!(agreement.all_equal && agreement.all_joins_exact);
    }

}
