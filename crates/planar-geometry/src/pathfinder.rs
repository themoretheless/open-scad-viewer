//! Illustrator-style Pathfinder ops on planar CAD rings.
//!
//! Built on [`crate::rings::planar`] (union / intersection / difference / xor).
//! Inputs are closed rings in document z-order: index 0 = back, last = front.
use crate::rings::{Rings, area, contains_point, planar};
use crate::{Result, check};

/// Max shapes for exponential arrangement walks (Divide / Shape Builder).
pub const MAX_ARRANGEMENT_INPUTS: usize = 10;
/// Max shapes for O(N²) Trim.
pub const MAX_TRIM_INPUTS: usize = 30;

/// One filled region: outer ring plus zero or more hole rings.
#[derive(Debug, Clone, PartialEq)]
pub struct Compound {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

impl Compound {
    pub fn rings(&self) -> Rings {
        let mut r = vec![self.outer.clone()];
        r.extend(self.holes.iter().cloned());
        r
    }

    pub fn from_rings(rings: Rings) -> Result<Vec<Self>> {
        classify_compounds(rings)
    }
}

fn ring_centroid(r: &[[f64; 2]]) -> [f64; 2] {
    let n = r.len().max(1) as f64;
    let sx: f64 = r.iter().map(|p| p[0]).sum();
    let sy: f64 = r.iter().map(|p| p[1]).sum();
    [sx / n, sy / n]
}

fn ring_contains_ring(outer: &[[f64; 2]], inner: &[[f64; 2]]) -> bool {
    // Sample several vertices; require a strict majority inside the outer.
    if inner.is_empty() {
        return false;
    }
    let hits = inner.iter().filter(|p| contains_point(**p, outer)).count();
    hits * 2 > inner.len()
}

/// Group flat boolean rings into compounds (outers + nested holes).
pub fn classify_compounds(mut rings: Rings) -> Result<Vec<Compound>> {
    rings.retain(|r| r.len() >= 3 && area(r).abs() > 1e-18);
    check(rings.len() <= 512, "Too many rings for compound classify")?;
    // Nesting depth via containment of centroids (stable for simple regions).
    let n = rings.len();
    let mut depth = vec![0usize; n];
    for i in 0..n {
        let c = ring_centroid(&rings[i]);
        for (j, other) in rings.iter().enumerate() {
            if i == j {
                continue;
            }
            if contains_point(c, other) && ring_contains_ring(other, &rings[i]) {
                depth[i] += 1;
            }
        }
    }
    let mut outers: Vec<usize> = (0..n).filter(|&i| depth[i].is_multiple_of(2)).collect();
    outers.sort_by(|&a, &b| area(&rings[b]).abs().total_cmp(&area(&rings[a]).abs()));
    let mut used_hole = vec![false; n];
    let mut compounds = Vec::new();
    for oi in outers {
        let mut holes = Vec::new();
        for hi in 0..n {
            if hi == oi || used_hole[hi] || depth[hi].is_multiple_of(2) {
                continue;
            }
            if ring_contains_ring(&rings[oi], &rings[hi]) {
                // Prefer the nearest outer: skip if another even-depth ring
                // sits between this hole and oi.
                let between = (0..n).any(|m| {
                    m != oi
                        && m != hi
                        && depth[m].is_multiple_of(2)
                        && ring_contains_ring(&rings[oi], &rings[m])
                        && ring_contains_ring(&rings[m], &rings[hi])
                });
                if !between {
                    used_hole[hi] = true;
                    holes.push(rings[hi].clone());
                }
            }
        }
        compounds.push(Compound {
            outer: rings[oi].clone(),
            holes,
        });
    }
    Ok(compounds)
}

fn validate_closed(rings: &Rings) -> Result<()> {
    check(!rings.is_empty(), "Empty region")?;
    check(
        rings.iter().all(|r| r.len() >= 3),
        "Ring needs at least 3 vertices",
    )?;
    check(
        rings
            .iter()
            .flat_map(|r| r.iter())
            .all(|p| p.iter().all(|x| x.is_finite() && x.abs() <= 1e6)),
        "Invalid pathfinder coordinate",
    )?;
    Ok(())
}

/// Pairwise Pathfinder: union / intersection / difference / xor.
pub fn combine(a: &Rings, b: &Rings, op: &str) -> Result<Rings> {
    validate_closed(a)?;
    validate_closed(b)?;
    let op = match op {
        "union" | "merge" => "union",
        "intersection" => "intersection",
        "difference" | "subtract" => "difference",
        "xor" | "exclude" => "xor",
        other => return Err(crate::error(format!("Unknown pathfinder op '{other}'"))),
    };
    planar(a, b, op)
}

pub fn xor(a: &Rings, b: &Rings) -> Result<Rings> {
    combine(a, b, "xor")
}

pub fn exclude(a: &Rings, b: &Rings) -> Result<Rings> {
    xor(a, b)
}

fn fold_op(shapes: &[Rings], op: &str) -> Result<Rings> {
    check(!shapes.is_empty(), "No shapes to combine")?;
    let mut acc = shapes[0].clone();
    for s in &shapes[1..] {
        acc = combine(&acc, s, op)?;
        if acc.is_empty() && op != "union" && op != "xor" {
            return Ok(acc);
        }
    }
    Ok(acc)
}

fn union_all(shapes: &[Rings]) -> Result<Rings> {
    fold_op(shapes, "union")
}

/// Minus Front (`keep_back`) / Minus Back (`!keep_back`).
/// Z-order: index 0 = back, last = front.
pub fn minus_by_zorder(shapes: &[Rings], keep_back: bool) -> Result<Rings> {
    check(shapes.len() >= 2, "Minus needs at least 2 shapes")?;
    for s in shapes {
        validate_closed(s)?;
    }
    let (survivor, cutters): (&Rings, Vec<&Rings>) = if keep_back {
        let (first, rest) = shapes.split_first().unwrap();
        (first, rest.iter().collect())
    } else {
        let (last, rest) = shapes.split_last().unwrap();
        (last, rest.iter().collect())
    };
    let cut = {
        super let cutter_rings: Vec<Rings> = cutters.iter().map(|c| (*c).clone()).collect();
        union_all(&cutter_rings)?
    };
    planar(survivor, &cut, "difference")
}

pub fn minus_front(shapes: &[Rings]) -> Result<Rings> {
    minus_by_zorder(shapes, true)
}

pub fn minus_back(shapes: &[Rings]) -> Result<Rings> {
    minus_by_zorder(shapes, false)
}

/// Crop: frontmost shape is the clip frame; each subject ∩ frame.
/// Returns one Rings result per subject that produced geometry (same order).
pub fn crop_to_front(shapes: &[Rings]) -> Result<Vec<Rings>> {
    check(shapes.len() >= 2, "Crop needs at least 2 shapes")?;
    for s in shapes {
        validate_closed(s)?;
    }
    let (frame, subjects) = shapes.split_last().unwrap();
    let mut out = Vec::new();
    for s in subjects {
        let piece = planar(s, frame, "intersection")?;
        if !piece.is_empty() {
            out.push(piece);
        }
    }
    check(!out.is_empty(), "Crop: nothing inside the front shape")?;
    Ok(out)
}

/// Trim: each shape minus the union of shapes in front of it.
/// Frontmost is always kept. Fully covered shapes are dropped (`None`).
pub fn trim_by_zorder(shapes: &[Rings]) -> Result<Vec<Option<Rings>>> {
    check(shapes.len() >= 2, "Trim needs at least 2 shapes")?;
    check(
        shapes.len() <= MAX_TRIM_INPUTS,
        "Trim: too many shapes at once",
    )?;
    for s in shapes {
        validate_closed(s)?;
    }
    let n = shapes.len();
    let mut out = Vec::with_capacity(n);
    let mut changed = false;
    for i in 0..n {
        if i + 1 == n {
            out.push(Some(shapes[i].clone()));
            continue;
        }
        let cutters: Vec<Rings> = shapes[i + 1..].to_vec();
        let cut = union_all(&cutters)?;
        let trimmed = planar(&shapes[i], &cut, "difference")?;
        if trimmed.is_empty() {
            changed = true;
            out.push(None);
        } else if ring_sets_equalish(&trimmed, &shapes[i]) {
            out.push(Some(shapes[i].clone()));
        } else {
            changed = true;
            out.push(Some(trimmed));
        }
    }
    check(changed, "Trim: nothing overlapped")?;
    Ok(out)
}

fn ring_sets_equalish(a: &Rings, b: &Rings) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let aa: f64 = a.iter().map(|r| area(r).abs()).sum();
    let bb: f64 = b.iter().map(|r| area(r).abs()).sum();
    (aa - bb).abs() <= 1e-6 * aa.max(bb).max(1.0)
}

/// Arrangement cells: each non-empty subset S → inside ∩S and outside ∪(¬S).
pub fn arrangement_cells(shapes: &[Rings]) -> Result<Vec<Rings>> {
    check(shapes.len() >= 2, "Arrangement needs at least 2 shapes")?;
    check(
        shapes.len() <= MAX_ARRANGEMENT_INPUTS,
        "Arrangement: too many shapes at once",
    )?;
    for s in shapes {
        validate_closed(s)?;
    }
    let n = shapes.len();
    let mut cells = Vec::new();
    for mask in 1u32..(1u32 << n) {
        let in_s: Vec<usize> = (0..n).filter(|&i| mask & (1u32 << i) != 0).collect();
        let inter = if in_s.len() == 1 {
            shapes[in_s[0]].clone()
        } else {
            let refs: Vec<Rings> = in_s.iter().map(|&i| shapes[i].clone()).collect();
            fold_op(&refs, "intersection")?
        };
        if inter.is_empty() {
            continue;
        }
        let others: Vec<usize> = (0..n).filter(|&i| mask & (1u32 << i) == 0).collect();
        if others.is_empty() {
            cells.push(inter);
            continue;
        }
        let rest_shapes: Vec<Rings> = others.iter().map(|&i| shapes[i].clone()).collect();
        let rest = union_all(&rest_shapes)?;
        if rest.is_empty() {
            cells.push(inter);
        } else {
            let frag = planar(&inter, &rest, "difference")?;
            if !frag.is_empty() {
                cells.push(frag);
            }
        }
    }
    Ok(cells)
}

/// Divide: flatten arrangement cells into independent fragment rings.
pub fn divide(shapes: &[Rings]) -> Result<Rings> {
    let cells = arrangement_cells(shapes)?;
    Ok(cells.into_iter().flatten().collect())
}

fn point_in_rings(p: [f64; 2], rings: &Rings) -> bool {
    // Non-zero: outer and holes cancel when classified as separate rings with
    // opposite winding after planar; use alternating containment count.
    let mut inside = false;
    let mut candidates: Vec<&Vec<[f64; 2]>> = rings.iter().collect();
    candidates.sort_by(|a, b| area(b).abs().total_cmp(&area(a).abs()));
    for r in candidates {
        if contains_point(p, r) {
            inside = !inside;
        }
    }
    inside
}

/// Shape Builder extract: return the arrangement cell containing `point`.
pub fn shape_builder_extract(shapes: &[Rings], point: [f64; 2]) -> Result<Rings> {
    check(
        point.iter().all(|x| x.is_finite() && x.abs() <= 1e6),
        "Invalid pick point",
    )?;
    let cells = arrangement_cells(shapes)?;
    for cell in cells {
        if point_in_rings(point, &cell) {
            return Ok(cell);
        }
    }
    Err(crate::error("Shape Builder: no cell under point"))
}

/// Shape Builder delete: subtract the cell under `point` from the union of inputs.
pub fn shape_builder_delete(shapes: &[Rings], point: [f64; 2]) -> Result<Rings> {
    let cell = shape_builder_extract(shapes, point)?;
    let all = union_all(shapes)?;
    planar(&all, &cell, "difference")
}

/// Make compound from several closed regions (union then classify holes).
pub fn make_compound(shapes: &[Rings]) -> Result<Vec<Compound>> {
    check(!shapes.is_empty(), "Make compound needs shapes")?;
    for s in shapes {
        validate_closed(s)?;
    }
    let merged = union_all(shapes)?;
    classify_compounds(merged)
}

/// Release compound into separate closed rings (outer + each hole as region).
pub fn release_compound(compound: &Compound) -> Rings {
    compound.rings()
}

/// Closed region tagged with a fill color for Merge-by-Color.
#[derive(Debug, Clone, PartialEq)]
pub struct ColoredRegion {
    pub rings: Rings,
    pub color: [u8; 4],
}

/// Union every closed region that shares the same RGBA key.
/// Front-most (last) appearance of a bucket wins the output color.
pub fn merge_by_color(shapes: &[ColoredRegion]) -> Result<Vec<ColoredRegion>> {
    check(shapes.len() >= 2, "Merge by color needs at least 2 shapes")?;
    for s in shapes {
        validate_closed(&s.rings)?;
    }
    let mut buckets: Vec<([u8; 4], Vec<Rings>)> = Vec::new();
    for s in shapes {
        if let Some((_, rings)) = buckets.iter_mut().find(|(c, _)| *c == s.color) {
            rings.push(s.rings.clone());
        } else {
            buckets.push((s.color, vec![s.rings.clone()]));
        }
    }
    check(
        buckets.iter().any(|(_, r)| r.len() >= 2),
        "Merge by color: no shared fill",
    )?;
    let mut out = Vec::new();
    for (color, group) in buckets {
        let rings = if group.len() == 1 {
            group.into_iter().next().unwrap()
        } else {
            union_all(&group)?
        };
        if !rings.is_empty() {
            out.push(ColoredRegion { rings, color });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> Rings {
        vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]]
    }

    #[test]
    fn xor_two_overlapping_squares() {
        let a = sq(0., 0., 4., 4.);
        let b = sq(2., 2., 6., 6.);
        let r = xor(&a, &b).unwrap();
        let area_sum: f64 = r.iter().map(|ring| area(ring).abs()).sum();
        // |A△B| = 16+16-2*4 = 24
        assert!((area_sum - 24.).abs() < 1e-6, "area={area_sum}");
    }

    #[test]
    fn divide_two_squares_three_cells() {
        let a = sq(0., 0., 4., 4.);
        let b = sq(2., 0., 6., 4.);
        let cells = arrangement_cells(&[a, b]).unwrap();
        assert!(cells.len() >= 3, "cells={}", cells.len());
        let frags = divide(&[sq(0., 0., 4., 4.), sq(2., 0., 6., 4.)]).unwrap();
        assert!(frags.len() >= 3);
    }

    #[test]
    fn crop_keeps_overlap_only() {
        let back = sq(0., 0., 4., 4.);
        let front = sq(3., 3., 5., 5.);
        let cropped = crop_to_front(&[back, front]).unwrap();
        assert_eq!(cropped.len(), 1);
        let a: f64 = cropped[0].iter().map(|r| area(r).abs()).sum();
        assert!((a - 1.).abs() < 1e-6, "area={a}");
    }

    #[test]
    fn trim_cuts_back_by_front() {
        let back = sq(0., 0., 4., 4.);
        let front = sq(0., 0., 2., 4.);
        let out = trim_by_zorder(&[back, front]).unwrap();
        assert!(out[0].is_some());
        assert!(out[1].is_some());
        let a: f64 = out[0].as_ref().unwrap().iter().map(|r| area(r).abs()).sum();
        assert!((a - 8.).abs() < 1e-6, "area={a}");
    }

    #[test]
    fn minus_front_keeps_back() {
        let back = sq(0., 0., 4., 4.);
        let front = sq(1., 1., 3., 3.);
        let r = minus_front(&[back, front]).unwrap();
        let a: f64 = r.iter().map(|ring| area(ring)).sum();
        assert!((a - 12.).abs() < 1e-6, "area={a}");
    }

    #[test]
    fn minus_back_keeps_front() {
        let back = sq(0., 0., 4., 4.);
        let front = sq(1., 1., 3., 3.);
        let r = minus_back(&[back.clone(), front.clone()]).unwrap();
        // front fully inside back → empty
        let a: f64 = r.iter().map(|ring| area(ring).abs()).sum();
        assert!(a < 1e-6, "area={a}");
        // side overhangs back by a 2×2 strip on the right
        let side = sq(3., 0., 5., 2.);
        let r2 = minus_back(&[back, side]).unwrap();
        let a2: f64 = r2.iter().map(|ring| area(ring)).sum();
        assert!((a2.abs() - 2.).abs() < 1e-6, "area={a2}");
    }

    #[test]
    fn compound_with_hole() {
        let outer = sq(0., 0., 4., 4.);
        let hole = sq(1., 1., 3., 3.);
        let diff = planar(&outer, &hole, "difference").unwrap();
        let compounds = classify_compounds(diff).unwrap();
        assert_eq!(compounds.len(), 1);
        assert_eq!(compounds[0].holes.len(), 1);
        let a: f64 = compounds[0].rings().iter().map(|r| area(r)).sum();
        assert!((a.abs() - 12.).abs() < 1e-6 || (a - 12.).abs() < 1e-6 || (a + 12.).abs() < 1e-6);
    }

    #[test]
    fn shape_builder_extract_and_delete() {
        let a = sq(0., 0., 4., 4.);
        let b = sq(2., 0., 6., 4.);
        let cell = shape_builder_extract(&[a.clone(), b.clone()], [3., 2.]).unwrap();
        assert!(!cell.is_empty());
        let left = shape_builder_extract(&[a.clone(), b.clone()], [1., 2.]).unwrap();
        let a_left: f64 = left.iter().map(|r| area(r).abs()).sum();
        assert!((a_left - 8.).abs() < 1e-6, "left={a_left}");
        let rest = shape_builder_delete(&[a, b], [1., 2.]).unwrap();
        let ar: f64 = rest.iter().map(|r| area(r).abs()).sum();
        // union area 24, delete left-only 8 → 16
        assert!((ar - 16.).abs() < 1e-6, "rest={ar}");
    }

    #[test]
    fn merge_by_color_unions_same_fill() {
        let a = ColoredRegion {
            rings: sq(0., 0., 2., 2.),
            color: [255, 0, 0, 255],
        };
        let b = ColoredRegion {
            rings: sq(1., 0., 3., 2.),
            color: [255, 0, 0, 255],
        };
        let c = ColoredRegion {
            rings: sq(10., 0., 11., 1.),
            color: [0, 0, 255, 255],
        };
        let out = merge_by_color(&[a, b, c]).unwrap();
        assert_eq!(out.len(), 2);
        let red = out.iter().find(|r| r.color == [255, 0, 0, 255]).unwrap();
        let a_red: f64 = red.rings.iter().map(|r| area(r).abs()).sum();
        assert!((a_red - 6.).abs() < 1e-6, "red={a_red}");
    }
}
