use std::collections::HashSet;

use crate::{Result, Sketch, identity_error};

/// Stable primitive identities stored alongside `Sketch`, preserving its literal-based public API.
/// IDs are globally unique across points, circles and constraints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentIds {
    pub points: Vec<u64>,
    pub circles: Vec<u64>,
    pub constraints: Vec<u64>,
}

/// New-index to old-index correspondence supplied by a parametric rebuild.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebuildMap {
    pub points: Vec<usize>,
    pub circles: Vec<usize>,
    pub constraints: Vec<usize>,
}

impl RebuildMap {
    pub fn identity(sketch: &Sketch) -> Self {
        Self {
            points: (0..sketch.points.len()).collect(),
            circles: (0..sketch.circles.len()).collect(),
            constraints: (0..sketch.constraints.len()).collect(),
        }
    }
}

impl PersistentIds {
    pub fn new(sketch: &Sketch) -> Result<Self> {
        sketch.validate()?;
        let total = sketch.points.len() + sketch.circles.len() + sketch.constraints.len();
        let mut ids = Self {
            points: Vec::with_capacity(sketch.points.len()),
            circles: Vec::with_capacity(sketch.circles.len()),
            constraints: Vec::with_capacity(sketch.constraints.len()),
        };
        for (offset, target) in [
            (0, &mut ids.points),
            (sketch.points.len(), &mut ids.circles),
            (
                sketch.points.len() + sketch.circles.len(),
                &mut ids.constraints,
            ),
        ] {
            let len = target.capacity();
            target.extend((1..=total).skip(offset).take(len).map(|id| id as u64));
        }
        Ok(ids)
    }

    pub fn validate_for(&self, sketch: &Sketch) -> Result<()> {
        sketch.validate()?;
        if self.points.len() != sketch.points.len()
            || self.circles.len() != sketch.circles.len()
            || self.constraints.len() != sketch.constraints.len()
        {
            return Err(identity_error("Persistent ID count does not match sketch"));
        }
        let unique: HashSet<_> = self
            .points
            .iter()
            .chain(&self.circles)
            .chain(&self.constraints)
            .copied()
            .collect();
        if unique.len() != self.points.len() + self.circles.len() + self.constraints.len()
            || unique.contains(&0)
        {
            return Err(identity_error(
                "Persistent IDs must be non-zero and globally unique",
            ));
        }
        Ok(())
    }

    /// Carries IDs through a rebuild using explicit semantic correspondence, including reordering.
    pub fn rebuild(&self, old: &Sketch, new: &Sketch, map: &RebuildMap) -> Result<Self> {
        self.validate_for(old)?;
        new.validate()?;
        let checks = [
            (map.points.len(), new.points.len(), "point"),
            (map.circles.len(), new.circles.len(), "circle"),
            (map.constraints.len(), new.constraints.len(), "constraint"),
        ];
        if checks
            .iter()
            .any(|(actual, expected, _)| actual != expected)
        {
            return Err(identity_error(
                "Rebuild map count does not match rebuilt sketch",
            ));
        }
        let remap = |indices: &[usize], ids: &[u64], kind: &str| -> Result<Vec<u64>> {
            indices
                .iter()
                .map(|&old_index| {
                    ids.get(old_index).copied().ok_or_else(|| {
                        identity_error(format!(
                            "Rebuild map has invalid old {kind} index {old_index}"
                        ))
                    })
                })
                .collect()
        };
        let rebuilt = Self {
            points: remap(&map.points, &self.points, "point")?,
            circles: remap(&map.circles, &self.circles, "circle")?,
            constraints: remap(&map.constraints, &self.constraints, "constraint")?,
        };
        rebuilt.validate_for(new)?;
        Ok(rebuilt)
    }
}
