//! Automatic all-face normal proposals. A found material chord can prove a
//! local thin region; exhausting this finite grid cannot prove wall safety.
use crate::{
    source_material_chord::{Certificate, Limits},
    source_volume::Body,
    source_wall_search,
};
use nurbs_core::{Error, Result};
pub struct Report<'a> {
    best: Option<Certificate<'a>>,
    minimum_mm: f64,
    pub attempts: usize,
    pub refused: usize,
    pub faces_visited: usize,
    pub faces_total: usize,
    pub proposals_exhausted: bool,
}
impl<'a> Report<'a> {
    pub fn best(&self) -> Option<&Certificate<'a>> {
        self.best.as_ref()
    }
    /// Strict upper witness below the threshold proves this local thin chord.
    pub fn thin_witness(&self) -> Option<&Certificate<'a>> {
        self.best
            .as_ref()
            .filter(|c| c.length_mm()[1] < self.minimum_mm)
    }
}
pub fn search<'a>(
    body: &'a Body,
    minimum_mm: f64,
    grid: usize,
    max_attempts: usize,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    if !minimum_mm.is_finite()
        || minimum_mm <= 0.
        || !(1..=8).contains(&grid)
        || !(1..=256).contains(&max_attempts)
    {
        return Err(Error::new(
            "BREP_SOURCE_WALL_SCAN_INPUT",
            "Choose positive wall threshold and bounded proposal work",
        ));
    }
    let faces_total = body.geometry().shell().faces().len();
    if faces_total < 2 {
        return Err(Error::new(
            "BREP_SOURCE_WALL_SCAN_INPUT",
            "Wall search requires at least two original faces",
        ));
    }
    let mut out = Report {
        best: None,
        minimum_mm,
        attempts: 0,
        refused: 0,
        faces_visited: 0,
        faces_total,
        proposals_exhausted: false,
    };
    for face in 0..faces_total {
        if out.attempts == max_attempts {
            break;
        }
        let others: Vec<_> = (0..faces_total).filter(|&f| f != face).collect();
        let r = source_wall_search::search(
            body,
            [&[face], &others],
            grid,
            max_attempts - out.attempts,
            tolerance_uv,
            Limits {
                cells: limits.cells,
                domain_cells: limits.domain_cells,
                normal_spans: limits.normal_spans,
                max_sine_squared: limits.max_sine_squared,
            },
        )?;
        out.attempts += r.attempts;
        out.refused += r.refused;
        out.faces_visited += 1;
        if let Some(c) = r.best {
            if out
                .best
                .as_ref()
                .is_none_or(|b| c.length_mm()[1] < b.length_mm()[1])
            {
                out.best = Some(c);
            }
        }
        if !r.candidates_exhausted {
            break;
        }
        out.proposals_exhausted = face + 1 == faces_total;
    }
    Ok(out)
}
