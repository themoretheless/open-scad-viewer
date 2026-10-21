//! Fresh G1 on explicitly scoped, actual saved B-rep edges.
use crate::{Model, Result};
use nurbs_core::{Error, surface_g1_pcurves, surface_join::Decision};
pub struct Seam {
    pub edge: usize,
    pub faces: [usize; 2],
    pub proof: surface_g1_pcurves::Report,
}
pub struct Report {
    pub seams: Vec<Seam>,
    pub within_tolerance: bool,
    pub cells: usize,
    pub exact_work: u64,
}
pub fn inspect_edges(
    model: &Model,
    edges: &[usize],
    normal_tolerance: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    model.validate()?;
    if edges.is_empty()
        || edges.len() > 256
        || max_cells > 100000
        || max_exact > 1000000
        || !normal_tolerance.is_finite()
        || !(0. ..=2.).contains(&normal_tolerance)
    {
        return Err(Error::new(
            "BREP_G1_SCOPE",
            "Need bounded actual edge scope and finite normal tolerance",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Report {
        seams: vec![],
        within_tolerance: true,
        cells: 0,
        exact_work: 0,
    };
    for &edge in edges {
        if edge >= model.edges.len() || !seen.insert(edge) || model.edges[edge].degenerate {
            return Err(Error::new(
                "BREP_G1_SCOPE",
                "Invalid/repeated/degenerate G1 edge",
            ));
        }
        let uses = model
            .faces
            .iter()
            .enumerate()
            .flat_map(|(f, face)| {
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(move |w| {
                        model.loops[w]
                            .coedges
                            .iter()
                            .filter(move |c| c.edge == edge)
                            .map(move |c| (f, c))
                    })
            })
            .collect::<Vec<_>>();
        let [(a, ca), (b, cb)] = uses.as_slice() else {
            return Err(Error::new(
                "BREP_G1_OWNERSHIP",
                "G1 edge needs exactly two saved face uses",
            ));
        };
        if a == b {
            return Err(Error::new(
                "BREP_G1_OWNERSHIP",
                "G1 supports must be distinct faces",
            ));
        }
        let proof = surface_g1_pcurves::inspect(
            &model.edges[edge].curve,
            [&ca.pcurve, &cb.pcurve],
            [&model.faces[*a].surface, &model.faces[*b].surface],
            [ca.reversed, cb.reversed],
            model.tolerance_mm,
            normal_tolerance,
            max_cells - out.cells,
            max_exact - out.exact_work,
        )?;
        out.cells += proof.cells;
        out.exact_work += proof.positions.exact_work;
        out.within_tolerance &= proof.decision == Decision::WithinTolerance;
        out.seams.push(Seam {
            edge,
            faces: [*a, *b],
            proof,
        });
    }
    Ok(out)
}
