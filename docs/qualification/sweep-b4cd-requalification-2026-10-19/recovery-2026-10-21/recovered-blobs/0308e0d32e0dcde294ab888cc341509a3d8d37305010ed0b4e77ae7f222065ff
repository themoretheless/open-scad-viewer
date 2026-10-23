//! Display mesh feature targets with area-weighted planar group centers.
use crate::{MeshView, Result, planar::Topology};
use geometry_ops::keypoints::{Geometry, Kind, Segment, length};
use math_core::{cross, sub};
pub fn geometry(mesh: MeshView<'_>, topology: &Topology) -> Result<Geometry> {
    mesh.validate()?;
    let points = points(mesh);
    let mut out = Geometry::default();
    for e in &topology.edges {
        crate::check(
            e.a < points.len() && e.b < points.len(),
            "Invalid keypoint edge vertices",
        )?;
        let a = points[e.a];
        let b = points[e.b];
        out.add(a, Kind::Vertex);
        out.add(b, Kind::Vertex);
        out.add(std::array::from_fn(|i| (a[i] + b[i]) / 2.), Kind::Midpoint);
        out.segments.push(Segment {
            a,
            b,
            interval: None,
        });
    }
    append_centers(&mut out, mesh, topology, &vec![true; topology.faces.len()])?;
    out.bounds_center(&points);
    Ok(out)
}
pub fn points(mesh: MeshView<'_>) -> Vec<[f64; 3]> {
    mesh.positions.as_chunks::<3>().0.to_vec()
}
pub fn append_centers(
    out: &mut Geometry,
    mesh: MeshView<'_>,
    topology: &Topology,
    accepted: &[bool],
) -> Result<()> {
    mesh.validate()?;
    crate::check(
        accepted.len() == topology.faces.len(),
        "Invalid keypoint face mask",
    )?;
    let points = points(mesh);
    for (face, accepted) in topology.faces.iter().zip(accepted) {
        if !accepted {
            continue;
        }
        let mut center = [0.; 3];
        let mut area = 0.;
        for &index in &face.triangles {
            crate::check(index < mesh.indices.len() / 3, "Invalid keypoint triangle")?;
            let t = &mesh.indices[index * 3..index * 3 + 3];
            let [a, b, c] = [points[t[0]], points[t[1]], points[t[2]]];
            let w = length(cross(sub(b, a), sub(c, a)));
            area += w;
            for i in 0..3 {
                center[i] += (a[i] + b[i] + c[i]) * w / 3.;
            }
        }
        if area > 1e-12 {
            out.add(center.map(|v| v / area), Kind::Center)
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planar_center_is_area_weighted_and_bounds_include_unused_vertices() {
        let positions = [0., 0., 0., 4., 0., 0., 0., 2., 0., 8., 4., 0.];
        let mesh = MeshView::new(&positions, &[0, 1, 2]);
        let topology = crate::planar::topology(mesh).unwrap();
        let g = geometry(mesh, &topology).unwrap();
        let center = g.points.iter().find(|p| p.kind == Kind::Center).unwrap();
        assert_eq!(center.point, [4. / 3., 2. / 3., 0.]);
        assert_eq!(g.points.last().unwrap().point, [4., 2., 0.]);
        let mut invalid = topology;
        invalid.faces[0].triangles.push(usize::MAX);
        assert!(geometry(mesh, &invalid).is_err());
    }
}
