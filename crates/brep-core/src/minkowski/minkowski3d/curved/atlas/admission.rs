//! Preserve the old successful proof recipe. A failed proof can retry with
//! whole-chart image identities, never with relaxed embedding predicates.
use super::*;
pub(super) fn checked(m: &Model, depth: u8) -> Result<(Certificate, Vec<Patch>)> {
    match checked_charts_recipe(m, depth, false) {
        Ok(c) => Ok(c),
        Err(original) => match checked_charts_recipe(m, depth, true) {
            Ok(c) => Ok(c),
            Err(_) => Err(original),
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn stationary_box() -> Model {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for face in &mut m.faces {
            let old = face.surface.control_points.clone();
            face.surface.degree_u = 3;
            face.surface.degree_v = 3;
            face.surface.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
            face.surface.knots_v = face.surface.knots_u.clone();
            let r = [0, 0, 1, 1];
            face.surface.control_points = (0..4)
                .map(|i| (0..4).map(|j| old[r[i]][r[j]].clone()).collect())
                .collect();
            face.surface.weights = vec![vec![1.; 4]; 4];
        }
        for edge in &mut m.edges {
            let a = edge.curve.control_points[0].clone();
            let b = edge.curve.control_points.last().unwrap().clone();
            edge.curve.degree = 3;
            edge.curve.knots = vec![0., 0., 0., 0., 1., 1., 1., 1.];
            edge.curve.control_points = vec![a.clone(), a, b.clone(), b];
            edge.curve.weights = vec![1.; 4];
        }
        m.rebuild_topology_ids();
        m.validate().unwrap();
        m
    }
    #[test]
    fn a_previously_unproved_stationary_brep_gets_a_complete_new_source_proof() {
        let m = stationary_box();
        assert!(checked_charts_recipe(&m, 0, false).is_err());
        let (c, _) = checked(&m, 0).unwrap();
        assert_eq!(c.triangles.len(), 12);
        super::super::super::verify_source_atlas(&m, &c).unwrap();
        let mut bad = m;
        let s = &bad.faces[0].surface;
        let axis = (0..3)
            .find(|k| {
                s.control_points
                    .iter()
                    .flatten()
                    .all(|p| p[*k] == s.control_points[0][0][*k])
            })
            .unwrap();
        let inward = if s.control_points[0][0][axis] == 0. {
            100.
        } else {
            -100.
        };
        bad.faces[0].surface.control_points[1][1][axis] += inward;
        bad.rebuild_topology_ids();
        assert!(checked(&bad, 0).is_err());
    }
}
