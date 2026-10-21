//! Native-only reuse of complete original levels, never caller-owned proofs.
//! Constructor/builder validation and the public count guard precede lookup.
//! Exact source bits include every law, frame, anchor and work policy. The
//! cached report retains its full original logical charge and refusal budget.
use super::{Curve, MultiLevel, MultiSweep, Options, Spacing, Sweep};
use std::{cell::RefCell, collections::VecDeque};
const MAX_ENTRIES: usize = 8;
const MAX_KEY_WORDS: usize = 16_384;
// Includes keys, control values and Vec/object metadata; excludes allocator
// bookkeeping. No key construction or retained payload can grow unbounded.
const MAX_WORDS: usize = 262_144;
struct Entry { key: Vec<u64>, level: MultiLevel, words: usize }
thread_local! { static CACHE: RefCell<VecDeque<Entry>> = const { RefCell::new(VecDeque::new()) }; }
struct Key(Vec<u64>);
impl Key {
    fn word(&mut self, word: u64) -> Option<()> {
        if self.0.len() == MAX_KEY_WORDS { return None; }
        self.0.push(word); Some(())
    }
    fn floats(&mut self, values: &[f64]) -> Option<()> {
        self.word(values.len() as u64)?;
        if values.len() > MAX_KEY_WORDS - self.0.len() { return None; }
        self.0.extend(values.iter().map(|v|v.to_bits())); Some(())
    }
    fn curve(&mut self, curve: &Curve) -> Option<()> {
        // Exhaustive destructuring makes new source fields a compile error.
        let Curve { degree, knots, control_points, weights, periodic } = curve;
        self.word(*degree as u64)?; self.word(*periodic as u64)?;
        self.floats(knots)?; self.word(control_points.len() as u64)?;
        for point in control_points { self.floats(point)?; }
        self.floats(weights)
    }
    fn optional_curve(&mut self, curve: Option<&Curve>) -> Option<()> {
        self.word(curve.is_some() as u64)?;
        if let Some(curve)=curve { self.curve(curve)?; } Some(())
    }
    fn options(&mut self, options: &Options) -> Option<()> {
        let Options { normal, orientation, spacing, initial_sections, max_sections, max_deviation } = options;
        self.floats(normal)?; self.word(*orientation as u64)?;
        match spacing {
            Spacing::Parameter => self.word(0)?,
            Spacing::ArcLength { tolerance, max_cells } => {
                self.word(1)?; self.word(tolerance.to_bits())?; self.word(*max_cells as u64)?;
            }
        }
        self.word(*initial_sections as u64)?; self.word(*max_sections as u64)?;
        self.word(max_deviation.to_bits())
    }
}
pub(super) fn key(multi: &MultiSweep<'_>, count: usize) -> Option<Vec<u64>> {
    let MultiSweep { sweeps, options, next_sections: _ } = multi;
    let mut key=Key(Vec::new()); key.word(count as u64)?; key.options(options)?;
    key.word(sweeps.len() as u64)?;
    for sweep in sweeps {
        // Station-cache state and iterator position cannot change an explicit
        // original preview. All geometric/source/policy fields are exhaustive.
        let Sweep { station_cache: _, profile, path, scale, twist, affine_laws,
            frame_laws, orientation_guide, contact_point, contact_source,
            spatial_rmf_error_limits, options, next_sections: _ } = sweep;
        for curve in [profile,path,scale,twist] { key.curve(curve)?; }
        for pair in [affine_laws,frame_laws] {
            key.word(pair.is_some() as u64)?;
            if let Some((a,b))=pair { key.curve(a)?; key.curve(b)?; }
        }
        key.optional_curve(*orientation_guide)?;
        key.word(contact_point.is_some() as u64)?;
        if let Some(point)=contact_point { key.floats(point)?; }
        key.word(contact_source.is_some() as u64)?;
        if let Some((curve,parameter))=contact_source { key.curve(curve)?; key.word(parameter.to_bits())?; }
        key.word(spatial_rmf_error_limits.is_some() as u64)?;
        if let Some(limits)=spatial_rmf_error_limits { for limit in limits { key.word(*limit as u64)?; } }
        key.options(options)?;
    }
    Some(key.0)
}
pub(super) fn get(key: &[u64]) -> Option<MultiLevel> {
    CACHE.with(|cache| {
        let entries=cache.try_borrow().ok()?;
        Some(entries.iter().find(|entry|entry.key==key)?.level.clone())
    })
}
fn words(level: &MultiLevel) -> Option<usize> {
    let mut n=std::mem::size_of::<MultiLevel>().div_ceil(8)
        .checked_add(level.profile_patch_ranges.len().checked_mul(2)?)?;
    let mut add=|count: usize| { n=n.checked_add(count)?; (n<=MAX_WORDS).then_some(()) };
    for surface in &level.patches {
        add(std::mem::size_of_val(surface).div_ceil(8))?;
        add(surface.knots_u.len())?; add(surface.knots_v.len())?;
        for row in &surface.control_points {
            add(3)?;
            for point in row { add(3)?; add(point.len())?; }
        }
        for row in &surface.weights { add(3)?; add(row.len())?; }
    }
    Some(n)
}
pub(super) fn put(key: Vec<u64>, level: &MultiLevel) {
    // Partial/unproved previews cannot acquire a cache entry or replace a
    // complete source result. Downstream admission still runs independently.
    if !level.report.continuous_bound { return; }
    let Some(words)=words(level).and_then(|n|n.checked_add(key.capacity()+8)).filter(|n|*n<=MAX_WORDS) else { return; };
    CACHE.with(|cache| {
        let Ok(mut entries)=cache.try_borrow_mut() else { return; };
        if entries.iter().any(|entry|entry.key==key) { return; }
        while entries.len()>=MAX_ENTRIES || entries.iter().map(|entry|entry.words).sum::<usize>()+words>MAX_WORDS {
            entries.pop_front();
        }
        entries.push_back(Entry {key,level:level.clone(),words});
    });
}
#[cfg(test)]
mod tests;
