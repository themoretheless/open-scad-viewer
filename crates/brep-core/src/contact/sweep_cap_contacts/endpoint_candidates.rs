use crate::Model;
/// Propose the two endpoint carrier charts emitted by retained section lofts.
/// This is a shape filter, not a certificate: the caller must still audit the
/// original planes, trims, every cap/wall contact and the complete shell.
/// Periodic walls with multiple stations are not endpoint carrier charts.
pub fn endpoint_chart_candidates(model: &Model) -> Vec<usize> {
    let Some(first) = model.faces.len().checked_sub(2) else {
        return Vec::new();
    };
    let candidates = [first, first + 1];
    if candidates.iter().all(|&face| {
        let s = &model.faces[face].surface;
        s.degree_u == 1
            && s.degree_v == 1
            && !s.periodic_u
            && !s.periodic_v
            && s.control_points.len() == 2
            && s.control_points.iter().all(|row| row.len() == 2)
    }) {
        // Endpoint caps cannot share a boundary edge. Two adjacent rectangular
        // faces (for example on a box) are not a pair of loft end carriers.
        let edges = |face: usize| {
            let f = &model.faces[face];
            std::iter::once(f.outer)
                .chain(f.holes.iter().copied())
                .flat_map(|index| model.loops[index].coedges.iter().map(|c| c.edge))
                .collect::<std::collections::BTreeSet<_>>()
        };
        if edges(first).is_disjoint(&edges(first + 1)) {
            candidates.to_vec()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_box_faces_are_not_endpoint_caps() {
        let model = crate::cuboid([0.; 3], [1., 2., 3.]).unwrap();
        let before = model.clone();
        assert!(endpoint_chart_candidates(&model).is_empty());
        assert_eq!(model, before);
    }

    #[test]
    fn separated_retained_loft_caps_remain_candidates() {
        let section = |z| {
            vec![vec![
                nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 1.).unwrap(),
            ]]
        };
        let model = crate::rational_section_loft(&[section(0.), section(2.)]).unwrap();
        assert_eq!(
            endpoint_chart_candidates(&model),
            vec![model.faces.len() - 2, model.faces.len() - 1]
        );
    }
}
