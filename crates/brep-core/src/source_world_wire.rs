//! Immutable incidence for original surface-composed edges.
//! Vertex ownership follows proven source joins, never proximity of world boxes.
use crate::source_boundary_fragment::Fragment;
use nurbs_core::{Error, Result};
#[derive(Clone)]
pub struct Wire {
    edges: Vec<Fragment>,
    vertices: Vec<[usize; 2]>,
}
pub struct Mapping {
    pub edge_bounds: Vec<[[f64; 2]; 3]>,
    pub cells: usize,
    pub complete: bool,
    pub uncertain_edge: Option<usize>,
}
impl Wire {
    pub fn new(edges: &[Fragment]) -> Result<Self> {
        if edges.is_empty() || edges.len() > 256 {
            return Err(Error::new(
                "BREP_SOURCE_WIRE",
                "Choose 1 through 256 original source edges",
            ));
        }
        for i in 0..edges.len() {
            if !edges[i].joins(&edges[(i + 1) % edges.len()]) {
                return Err(Error::new(
                    "BREP_SOURCE_WIRE",
                    "Original source vertex identity is unproven",
                ));
            }
        }
        Ok(Self {
            edges: edges.to_vec(),
            vertices: (0..edges.len())
                .map(|i| [i, (i + 1) % edges.len()])
                .collect(),
        })
    }
    pub fn edges(&self) -> &[Fragment] {
        &self.edges
    }
    /// Start/end vertex indices for each directed original edge.
    pub fn vertices(&self) -> &[[usize; 2]] {
        &self.vertices
    }
    pub fn world_mapping(&self, max_cells: usize) -> Result<Mapping> {
        if !(1..=100000).contains(&max_cells) {
            return Err(Error::new(
                "BREP_SOURCE_WIRE",
                "Choose bounded shared world mapping work",
            ));
        }
        let mut out = Mapping {
            edge_bounds: vec![],
            cells: 0,
            complete: false,
            uncertain_edge: None,
        };
        for (i, edge) in self.edges.iter().enumerate() {
            if out.cells == max_cells {
                out.uncertain_edge = Some(i);
                return Ok(out);
            }
            let report = edge.world_enclosure(max_cells - out.cells)?;
            out.cells += report.cells;
            let Some(bounds) = report.world_box else {
                out.uncertain_edge = Some(i);
                return Ok(out);
            };
            out.edge_bounds.push(bounds);
        }
        out.complete = true;
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::Endpoint;
    use nurbs_core::{curve::Curve, surface::Surface};
    fn edges() -> Vec<Fragment> {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 1.]],
                vec![vec![1., 0., 1.], vec![1., 1., 2.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        (0..4)
            .map(|i| {
                let c = Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap();
                Fragment::new(&s, &c, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap()
            })
            .collect()
    }
    #[test]
    fn incidence_requires_exact_source_joins_and_mapping_shares_work() {
        let e = edges();
        let w = Wire::new(&e).unwrap();
        assert_eq!(w.vertices(), &[[0, 1], [1, 2], [2, 3], [3, 0]]);
        let r = w.world_mapping(8).unwrap();
        assert!(r.complete && r.cells == 8 && r.edge_bounds.len() == 4);
        let r = w.world_mapping(5).unwrap();
        assert!(!r.complete && r.cells == 5 && r.uncertain_edge == Some(2));
        assert_eq!(r.edge_bounds.len(), 2);
        assert!(Wire::new(&e[..3]).is_err());
        let mut bad = e.clone();
        let mut c = bad[1].curve().clone();
        c.control_points[0][0] = 1. - 1e-12;
        bad[1] = Fragment::new(
            bad[1].surface(),
            &c,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        assert!(Wire::new(&bad).is_err());
    }
}
