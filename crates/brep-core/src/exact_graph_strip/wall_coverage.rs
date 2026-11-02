//! Whole-body lower bound for normal-aligned ENTERING/EXITING material chords.
//! Not an unconstrained minimum distance, a normal-cone edge metric or witness.
use super::*;
use nurbs_core::original_graph_transition::{
    SupportFrame, frame_distance_at_least, opposed_normal_chords_excluded, polynomial_support_frame,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignedWallPairDecision {
    DistanceLowerBound,
    OpposedNormalsExcluded,
    Unresolved,
}
pub struct AlignedWallPair {
    pub faces: [usize; 2],
    pub decision: AlignedWallPairDecision,
}
pub struct AlignedWallCoverage<'a> {
    body: &'a NativeVolume,
    minimum: Q,
    max_sine_squared: f64,
    pairs: Vec<AlignedWallPair>,
    pub(super) frames: Vec<SupportFrame>,
}
impl<'a> AlignedWallCoverage<'a> {
    pub fn body(&self) -> &'a NativeVolume {
        self.body
    }
    pub fn minimum_mm(&self) -> &Q {
        &self.minimum
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn pairs(&self) -> &[AlignedWallPair] {
        &self.pairs
    }
    pub fn complete(&self) -> bool {
        !self.pairs.is_empty()
            && self
                .pairs
                .iter()
                .all(|p| p.decision != AlignedWallPairDecision::Unresolved)
    }
}
impl NativeVolume {
    /// Every face product, INCLUDING SAME-FACE diagonal neighborhoods, is
    /// either distance-qualified, normal-opposition-excluded or unresolved.
    /// Reuses only this immutable admitted volume's closed embedded outward
    /// boundary. Positive material chords enter at one end and exit at the
    /// other, so their aligned outward normals must be nearly opposite.
    pub fn inspect_aligned_wall_lower_bound<'a>(
        &'a self,
        minimum: Q,
        max_sine_squared: f64,
        max_pairs: usize,
        b: &mut Budget,
    ) -> Result<AlignedWallCoverage<'a>> {
        require(
            minimum > Q::zero()
                && minimum.numer().bits() <= 8192
                && minimum.denom().bits() <= 8192
                && max_sine_squared.is_finite()
                && (0. ..0.5).contains(&max_sine_squared),
            "Choose a positive exact wall bound and an endpoint-normal angle below pi/4",
        )?;
        let top = self.topology();
        let n = top.faces.len();
        let total = n
            .checked_mul(n + 1)
            .and_then(|x| x.checked_div(2))
            .ok_or_else(|| Error::new("BREP_ALIGNED_WALL", "Face product count overflow"))?;
        require(
            total <= max_pairs && max_pairs <= 1_000_000,
            "Aligned wall product budget exhausted",
        )?;
        let mut frames: Vec<SupportFrame> = Vec::new();
        for (i, f) in top.faces.iter().enumerate() {
            let use_ = top.shells[0]
                .faces
                .iter()
                .find(|u| u.face == i)
                .ok_or_else(|| {
                    Error::new(
                        "BREP_ALIGNED_WALL",
                        "Face is not owned by the admitted boundary",
                    )
                })?;
            frames.push(match &f.surface {
                FaceGeometry::Transition(p) => p.support_frame(use_.reversed, b)?,
                FaceGeometry::Retained(r) => {
                    polynomial_support_frame(r.source(), use_.reversed, b)?
                }
                FaceGeometry::Imported(f) => {
                    polynomial_support_frame(f.source(), use_.reversed, b)?
                }
            });
        }
        let mut pairs = Vec::with_capacity(total);
        for i in 0..n {
            for j in i..n {
                let decision = if frame_distance_at_least(&frames[i], &frames[j], &minimum, b)? {
                    AlignedWallPairDecision::DistanceLowerBound
                } else if let (Some(a), Some(z)) = (frames[i].normal(), frames[j].normal()) {
                    if opposed_normal_chords_excluded(a, z, max_sine_squared)? {
                        AlignedWallPairDecision::OpposedNormalsExcluded
                    } else {
                        AlignedWallPairDecision::Unresolved
                    }
                } else {
                    AlignedWallPairDecision::Unresolved
                };
                pairs.push(AlignedWallPair {
                    faces: [i, j],
                    decision,
                });
            }
        }
        Ok(AlignedWallCoverage {
            body: self,
            minimum,
            max_sine_squared,
            pairs,
            frames,
        })
    }
}
