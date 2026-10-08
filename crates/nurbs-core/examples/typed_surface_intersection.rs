use nurbs_core::{
    Result,
    ss_intersection::{self, SurfaceSurfaceComponent},
    surface::Surface,
};

fn plane(vertical: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        if vertical {
                            vec![u as f64, 0., v as f64]
                        } else {
                            vec![u as f64, v as f64, 0.]
                        }
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn main() -> Result<()> {
    let first = plane(false);
    let report = ss_intersection::intersect_surface_surface_report(&first, &plane(true), None)?;
    assert!(report.complete());
    assert_eq!(report.components.len(), 1);
    let summary = report.summary();
    assert_eq!(summary.component_count, 1);
    assert!(summary.permits_topology_authorship);
    let SurfaceSurfaceComponent::Exact(branch) = &report.components[0] else {
        panic!("Expected an affine intersection branch");
    };
    assert_eq!(branch.samples.len(), 3);
    println!(
        "Intersection endpoints: {:?} -> {:?}",
        branch.samples[0].point, branch.samples[2].point
    );
    let overlap = ss_intersection::intersect_surface_surface_report(&first, &first, None)?;
    assert!(matches!(
        overlap.components.as_slice(),
        [SurfaceSurfaceComponent::Overlap(_)]
    ));
    assert_eq!(overlap.summary().overlap_regions, 1);
    Ok(())
}
