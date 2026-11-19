//! Exact knot-line transport through a partition of the original tensor chart.
use super::super::{graph_jets, periodic_uses};
use crate::{Result, Surface};
use nurbs_core::{
    continuity::ProjectiveSeam, editing::exact_surface_knot_insertion::tile_tensor_surface,
};

fn cuts(knots: &[f64], degree: usize, count: usize, order: usize) -> Option<Vec<f64>> {
    let [lo, hi] = [knots[degree], knots[count]];
    if knots[..=degree].iter().any(|&k| k != lo) || knots[count..].iter().any(|&k| k != hi) {
        return None;
    }
    let mut cuts = Vec::new();
    for block in knots
        .chunk_by(|a, b| a == b)
        .filter(|k| lo < k[0] && k[0] < hi)
    {
        if block.len() > degree {
            return None;
        }
        if degree < block.len() + order {
            cuts.push(block[0]);
        }
    }
    (cuts.len() <= 254).then_some(cuts)
}

pub(super) struct Report {
    pub(super) certified: bool,
    pub(super) cells: usize,
    pub(super) work: u64,
    pub(super) seams: usize,
}

pub(super) fn inspect(surface: &Surface, order: usize, cells: usize, work: u64) -> Result<Report> {
    let mut report = Report {
        certified: false,
        cells: 0,
        work: 0,
        seams: 0,
    };
    if work == 0 || cells == 0 {
        return Ok(report);
    }
    let wraps = [surface.periodic_u, surface.periodic_v];
    let (surface, used) = periodic_uses::open(surface, work)?;
    report.work = used;
    let Some(surface) = surface else {
        return Ok(report);
    };
    inspect_opened(&surface, wraps, order, cells, work - report.work).map(|mut proof| {
        proof.work += report.work;
        proof
    })
}

pub(super) fn inspect_opened(
    surface: &Surface, wraps: [bool; 2], order: usize, cells: usize, work: u64,
) -> Result<Report> {
    let mut report = Report { certified: false, cells: 0, work: 0, seams: 0 };
    if work == 0 || cells == 0 {
        return Ok(report);
    }
    let (Some(u), Some(v)) = (
        cuts(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
            order,
        ),
        cuts(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
            order,
        ),
    ) else {
        return Ok(report);
    };
    let [columns, rows] = [u.len() + 1, v.len() + 1];
    let count = rows * columns;
    let seam_count =
        rows * (u.len() + usize::from(wraps[0])) + columns * (v.len() + usize::from(wraps[1]));
    if seam_count == 0 || count > cells || count > 4096 || seam_count > 4096 {
        return Ok(report);
    }
    report.cells = count;
    let patches = if count == 1 {
        vec![surface.clone()]
    } else {
        let remaining = (work - report.work).min(cad_predicates::MAX_WORK);
        if remaining == 0 {
            return Ok(report);
        }
        let tiled = tile_tensor_surface(surface, &u, &v, remaining)?;
        report.work += tiled.work_used;
        let Some(patches) = tiled.patches else {
            return Ok(report);
        };
        patches
    };

    let mut seams = Vec::with_capacity(seam_count);
    let mut join = |a, b, ab: &str, bb: &str| {
        seams.push(ProjectiveSeam {
            patches: [a, b],
            boundaries: [ab.into(), bb.into()],
            order,
            normal_scale: 1.,
        });
    };
    for row in 0..rows {
        for column in 0..columns {
            let index = row * columns + column;
            if column + 1 < columns {
                join(index, index + 1, "uMax", "uMin");
            } else if wraps[0] {
                join(index, row * columns, "uMax", "uMin");
            }
            if row + 1 < rows {
                join(index, index + columns, "vMax", "vMin");
            } else if wraps[1] {
                join(index, column, "vMax", "vMin");
            }
        }
    }
    let proof = graph_jets::qualify_with_chart_proof(
        &patches,
        &seams,
        work - report.work,
        graph_jets::ChartProof::Parametric,
    )?;
    report.work += proof.exact_work;
    report.certified = proof.certified;
    if proof.certified {
        report.seams = seams.len();
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
