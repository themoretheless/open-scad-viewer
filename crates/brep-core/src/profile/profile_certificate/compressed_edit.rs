//! Transactional edit of one wire in a planar material profile.
use super::{certify, refused, verify, VerifiedProfile, MAX_SPANS};
use crate::{Curve, Result};
use nurbs_core::involute::LosslessSectorPattern;

/// Certify the complete proposed wire together with the supplied companion
/// wires (e.g. bores). Only then publish the replacement to the compressed wire.
pub fn replace_certified_profile_sector(
    wire: &mut LosslessSectorPattern, index: usize, replacement: &[Curve],
    companions: &[Vec<Curve>], tolerance: f64,
) -> Result<VerifiedProfile> {
    if index >= wire.sector_count() || replacement.is_empty() {
        return Err(refused("Expected a complete sector at a valid index"));
    }
    let curve_count = companions.iter().fold(
        wire.sector_count().saturating_mul(replacement.len()),
        |total, other| total.saturating_add(other.len()),
    );
    if curve_count > MAX_SPANS || replacement.len() != wire.sector(0)?.len() {
        return Err(refused("Profile exceeds certificate budget or sector layout differs"));
    }
    let mut outline = Vec::new();
    for i in 0..wire.sector_count() {
        if i == index { outline.extend_from_slice(replacement); }
        else { outline.extend(wire.sector(i)?); }
    }
    let mut loops = vec![outline];
    loops.extend_from_slice(companions);
    let certificate = certify(&loops, tolerance)?;
    let verified = verify(&loops, tolerance, &certificate)?;
    wire.replace_sector_with_closed_joins(index, replacement)?;
    Ok(verified)
}
