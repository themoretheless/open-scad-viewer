//! Screen-dependent selection of existing tessellation LODs.
//!
//! `error_upper` is supplied by each already-built level. This selector does
//! not estimate or certify Hausdorff distance and never synthesizes a level.
use crate::{Error, Result, check};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    pub triangle_count: usize,
    /// Geometric error upper bound in model units, owned by the LOD producer.
    pub error_upper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Request {
    /// Screen pixels per model unit at the surface's viewing depth.
    pub pixels_per_unit: f64,
    pub pixel_budget: f64,
    pub triangle_budget: usize,
    /// Upgrade above `pixel_budget * hysteresis`, downgrade below
    /// `pixel_budget / hysteresis`. One disables the hold band.
    pub hysteresis: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Initial,
    PixelBudget,
    HysteresisHold,
    TriangleBudget,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selection {
    pub level: usize,
    pub projected_error: f64,
    pub within_pixel_budget: bool,
    pub reason: Reason,
}

/// Levels are ordered coarse to fine, with strictly increasing triangle counts
/// and strictly decreasing error bounds. Budget-forced changes bypass the
/// hysteresis band. At least one existing level must fit the triangle budget.
pub fn select(levels: &[Level], current: Option<usize>, request: Request) -> Result<Selection> {
    check(
        (2..=3).contains(&levels.len()),
        "Viewer LOD selection requires two or three existing levels",
    )?;
    check(
        request.pixels_per_unit.is_finite()
            && request.pixels_per_unit > 0.
            && request.pixel_budget.is_finite()
            && request.pixel_budget > 0.
            && request.hysteresis.is_finite()
            && (1.0..=100.0).contains(&request.hysteresis)
            && request.triangle_budget > 0,
        "Viewer LOD requires positive finite screen/budget inputs and hysteresis 1..100",
    )?;
    for level in levels {
        check(
            level.triangle_count > 0 && level.error_upper.is_finite() && level.error_upper >= 0.,
            "Viewer LOD levels require triangles and finite nonnegative error bounds",
        )?;
    }
    check(
        levels.windows(2).all(|pair| {
            pair[0].triangle_count < pair[1].triangle_count
                && pair[0].error_upper > pair[1].error_upper
        }),
        "Viewer LOD levels must be strictly ordered coarse to fine",
    )?;
    let eligible: Vec<usize> = levels
        .iter()
        .enumerate()
        .filter_map(|(index, level)| {
            (level.triangle_count <= request.triangle_budget).then_some(index)
        })
        .collect();
    if eligible.is_empty() {
        return Err(Error::new(
            "NURBS_LOD_TRIANGLE_BUDGET",
            "No existing viewer LOD fits the triangle budget",
        ));
    }
    let projected = |index: usize| levels[index].error_upper * request.pixels_per_unit;
    let finest_within = |limit: f64| {
        eligible
            .iter()
            .copied()
            .filter(|&index| projected(index) <= limit)
            .last()
            .unwrap_or(eligible[0])
    };
    let base = finest_within(request.pixel_budget);
    let current_valid = current.filter(|&index| eligible.contains(&index));
    let (level, reason) = if let Some(current) = current_valid {
        if projected(current) > request.pixel_budget * request.hysteresis {
            (base, Reason::PixelBudget)
        } else if current > 0
            && projected(current) < request.pixel_budget / request.hysteresis
            && projected(current - 1) <= request.pixel_budget
        {
            // One transition per query prevents a fine level from oscillating
            // through the whole chain when the camera jumps. The coarser level
            // must itself fit the budget, otherwise the next query upgrades
            // straight back and the selection cycles.
            (current - 1, Reason::PixelBudget)
        } else {
            (current, Reason::HysteresisHold)
        }
    } else {
        (
            base,
            if current.is_some() {
                Reason::TriangleBudget
            } else {
                Reason::Initial
            },
        )
    };
    Ok(Selection {
        level,
        projected_error: projected(level),
        within_pixel_budget: projected(level) <= request.pixel_budget,
        reason,
    })
}
