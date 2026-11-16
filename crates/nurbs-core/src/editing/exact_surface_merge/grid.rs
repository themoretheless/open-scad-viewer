//! Exact coalescence of Cartesian patch grids with transverse basis alignment.
use super::{
    strip::{StripRefusal, merge_surface_strip},
    *,
};
use crate::{check, editing::exact_surface_knot_insertion};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridRefusal {
    ResourceLimit,
    Row {
        index: usize,
        reason: StripRefusal,
    },
    CrossRowBasis {
        index: usize,
    },
    BasisInsertion {
        index: usize,
        reason: exact_surface_knot_insertion::Refusal,
    },
    ColumnJoin(StripRefusal),
}

#[derive(Clone, Debug)]
pub struct GridMerge {
    /// Complete chart only. An unsuccessful join never publishes a partial strip.
    pub surface: Option<Surface>,
    pub inserted_knots_for_alignment: usize,
    pub work_used: u64,
    pub refusal: Option<GridRefusal>,
}

/// Merge a rectangular grid of compatible patches, in V-major order with U
/// varying fastest. Rows may remove different U knots. The exact union of
/// their remaining U bases is inserted before joining in V. Original UV
/// domains and rational denominators are preserved within one work ledger.
pub fn merge_surface_grid(patches: &[Surface], columns: usize, max_work: u64) -> Result<GridMerge> {
    check(
        columns >= 2 && patches.len().is_multiple_of(columns) && patches.len() / columns >= 2,
        "Exact patch grid needs two or more rows and columns",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact grid merge work must be in [1,1000000]",
    )?;
    let mut report = GridMerge {
        surface: None,
        inserted_knots_for_alignment: 0,
        work_used: 0,
        refusal: None,
    };
    let result = merge_grid(patches, columns, max_work, &mut report)?;
    match result {
        Ok(surface) => report.surface = Some(surface),
        Err(reason) => report.refusal = Some(reason),
    }
    Ok(report)
}

fn merge_grid(
    patches: &[Surface],
    columns: usize,
    max_work: u64,
    report: &mut GridMerge,
) -> Result<std::result::Result<Surface, GridRefusal>> {
    let mut rows = Vec::new();
    for (index, patches) in patches.chunks_exact(columns).enumerate() {
        let remaining = max_work - report.work_used;
        if remaining == 0 {
            return Ok(Err(GridRefusal::ResourceLimit));
        }
        let row = merge_surface_strip(patches, Axis::U, remaining)?;
        report.work_used += row.work_used;
        let Some(surface) = row.surface else {
            return Ok(Err(GridRefusal::Row {
                index,
                reason: row.refusal.unwrap_or(StripRefusal::ResourceLimit),
            }));
        };
        rows.push(surface);
    }

    let degree = rows[0].degree_u;
    let range = u_range(&rows[0]);
    for (index, row) in rows.iter().enumerate() {
        if row.degree_u != degree || u_range(row) != range {
            return Ok(Err(GridRefusal::CrossRowBasis { index }));
        }
    }
    let Some(union) = union_knots(&rows, range, degree) else {
        return Ok(Err(GridRefusal::ResourceLimit));
    };
    for (index, row) in rows.iter_mut().enumerate() {
        let mut knots = row.knots_u.iter().copied().peekable();
        let mut missing = Vec::new();
        for &(knot, multiplicity) in &union {
            while knots.next_if(|&u| u < knot).is_some() {}
            let mut current = 0;
            while knots.next_if_eq(&knot).is_some() {
                current += 1;
            }
            if current < multiplicity {
                missing.push((knot, multiplicity - current));
            }
        }
        for (knot, count) in missing {
            let remaining = max_work - report.work_used;
            if remaining == 0 {
                return Ok(Err(GridRefusal::ResourceLimit));
            }
            let inserted =
                exact_surface_knot_insertion::insert(row, Axis::U, knot, count, remaining)?;
            report.work_used += inserted.work_used;
            if let Some(reason) = inserted.refusal {
                return Ok(Err(GridRefusal::BasisInsertion { index, reason }));
            }
            report.inserted_knots_for_alignment += inserted.inserted;
            *row = inserted.surface;
        }
    }
    let remaining = max_work - report.work_used;
    if remaining == 0 {
        return Ok(Err(GridRefusal::ResourceLimit));
    }
    let joined = merge_surface_strip(&rows, Axis::V, remaining)?;
    report.work_used += joined.work_used;
    Ok(joined.surface.ok_or_else(|| {
        GridRefusal::ColumnJoin(joined.refusal.unwrap_or(StripRefusal::ResourceLimit))
    }))
}

fn u_range(surface: &Surface) -> [f64; 2] {
    [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ]
}

fn union_knots(
    rows: &[Surface],
    [start, end]: [f64; 2],
    degree: usize,
) -> Option<Vec<(f64, usize)>> {
    // Float bits key only the exact equality classes. Normalize signed zero
    // and sort the small result numerically after collecting multiplicities.
    let mut values = std::collections::BTreeMap::new();
    let mut controls = degree + 1;
    for row in rows {
        for run in row.knots_u.chunk_by(|a, b| a == b) {
            let knot = run[0];
            if knot <= start || knot >= end {
                continue;
            }
            let key = if knot == 0. { 0 } else { knot.to_bits() };
            let entry = values.entry(key).or_insert((knot, 0));
            if run.len() > entry.1 {
                controls += run.len() - entry.1;
                if controls > 256 {
                    return None;
                }
                entry.1 = run.len();
            }
        }
    }
    let mut values = values.into_values().collect::<Vec<_>>();
    values.sort_by(|(a, _), (b, _)| a.total_cmp(b));
    Some(values)
}
