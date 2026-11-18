//! Lazy rotational copies of a rational tooth-profile segment about the Z axis.
use super::{certified_pi, quintic_certificate::trig};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};

pub struct CircularProfilePattern {
    seed: Curve,
    count: usize,
}

impl CircularProfilePattern {
    pub fn new(seed: Curve, count: usize) -> Result<Self> {
        seed.validate()?;
        check(matches!(seed.control_points[0].len(), 2 | 3), "Expected XY or XYZ profile")?;
        check((1..=65536).contains(&count), "Expected 1..65536 rotational copies")?;
        Ok(Self { seed, count })
    }

    /// The bound covers the entire returned rational curve relative to the
    /// ideal rotation of the stored seed, independently of any seed fitting error.
    pub fn instance(&self, index: usize) -> Result<(Curve, f64)> {
        check(index < self.count, "Rotational instance index outside pattern")?;
        let quarter_steps = 4 * index;
        let mut curve = self.seed.clone();
        if quarter_steps % self.count == 0 {
            for pole in &mut curve.control_points {
                let (x, y) = (pole[0], pole[1]);
                (pole[0], pole[1]) = match quarter_steps / self.count {
                    0 => (x, y), 1 => (-y, x), 2 => (-x, -y), _ => (y, -x),
                };
            }
            return Ok((curve, 0.));
        }
        let angle = certified_pi()?.mul(I::point((2 * index) as f64))?.div(I::point(self.count as f64))?;
        let (sin, cos) = trig(angle)?;
        let mut error: f64 = 0.;
        for pole in &mut curve.control_points {
            let (x, y) = (I::point(pole[0]), I::point(pole[1]));
            let bounds = [x.mul(cos)?.sub(y.mul(sin)?)?, x.mul(sin)?.add(y.mul(cos)?)?];
            let mut pole_error = I::point(0.);
            for (coordinate, range) in pole.iter_mut().zip(bounds) {
                *coordinate = range.lo.midpoint(range.hi);
                let delta = range.sub(I::point(*coordinate))?;
                pole_error = pole_error.add(I::point(delta.lo.abs().max(delta.hi.abs())))?;
            }
            // Positive rational basis weights form a convex combination of pole errors.
            error = error.max(pole_error.hi);
        }
        Ok((curve, error))
    }
}

impl CircularProfilePattern {
    /// Compress ordered, equally spaced segments with the same rational basis.
    /// The bound compares each stored input with the actual reconstructed curve
    /// at the same parameter. A zero tolerance requires identical pole coordinates.
    pub fn compress(segments: &[Curve], tolerance: f64) -> Result<Option<(Self, f64)>> {
        check(tolerance.is_finite() && tolerance >= 0., "Expected finite nonnegative tolerance")?;
        check((1..=65536).contains(&segments.len()), "Expected 1..65536 profile segments")?;
        for segment in segments { segment.validate()?; }
        Self::compress_column(segments, 0, 1, tolerance)
    }

    fn compress_column(segments: &[Curve], offset: usize, stride: usize, tolerance: f64)
        -> Result<Option<(Self, f64)>> {
        let seed = &segments[offset];
        let pattern = Self::new(seed.clone(), segments.len() / stride)?;
        let mut maximum: f64 = 0.;
        for (index, segment) in segments[offset..].iter().step_by(stride).enumerate().skip(1) {
            if segment.degree != seed.degree || segment.knots != seed.knots ||
                segment.weights != seed.weights || segment.periodic != seed.periodic ||
                segment.control_points[0].len() != seed.control_points[0].len() {
                return Ok(None);
            }
            let (reconstructed, _) = pattern.instance(index)?;
            for (a, b) in reconstructed.control_points.iter().zip(&segment.control_points) {
                let mut displacement = I::point(0.);
                for (&x, &y) in a.iter().zip(b) {
                    if x == y { continue; }
                    if tolerance == 0. { return Ok(None); }
                    let delta = I::point(x).sub(I::point(y))?;
                    displacement = displacement.add(I::point(delta.lo.abs().max(delta.hi.abs())))?;
                }
                maximum = maximum.max(displacement.hi);
                if maximum > tolerance { return Ok(None); }
            }
        }
        Ok(Some((pattern, maximum)))
    }
}

/// Repeated sectors of an ordered profile. Different pieces within a sector
/// may have different degrees, knots and weights.
pub struct CircularSectorPattern {
    pieces: Vec<CircularProfilePattern>,
    sector_count: usize,
}

impl CircularSectorPattern {
    /// Input order is sector-major. The caller supplies the number of pieces
    /// per sector; the bound applies pointwise to each reconstructed piece.
    pub fn compress(segments: &[Curve], pieces_per_sector: usize, tolerance: f64)
        -> Result<Option<(Self, f64)>> {
        check(tolerance.is_finite() && tolerance >= 0., "Expected finite nonnegative tolerance")?;
        check(pieces_per_sector > 0 && !segments.is_empty() && segments.len() % pieces_per_sector == 0,
              "Expected complete nonempty profile sectors")?;
        let sector_count = segments.len() / pieces_per_sector;
        check(sector_count <= 65536, "Expected at most 65536 profile sectors")?;
        for segment in segments { segment.validate()?; }
        Self::compress_validated(segments, pieces_per_sector, tolerance)
    }

    /// Find the shortest reconstructible sector in an ordered outline. A match
    /// requires at least two copies; a single copy of the whole outline is excluded.
    pub fn discover(segments: &[Curve], tolerance: f64) -> Result<Option<(Self, f64)>> {
        check(!segments.is_empty(), "Expected a nonempty profile")?;
        check(tolerance.is_finite() && tolerance >= 0., "Expected finite nonnegative tolerance")?;
        for segment in segments {
            segment.validate()?;
            check(matches!(segment.control_points[0].len(), 2 | 3), "Expected XY or XYZ profile")?;
        }
        for count in (2..=segments.len().min(65536)).rev() {
            if segments.len() % count != 0 { continue; }
            if let Some(found) = Self::compress_validated(segments, segments.len()/count, tolerance)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    pub fn sector_count(&self) -> usize { self.sector_count }

    fn compress_validated(segments: &[Curve], pieces_per_sector: usize, tolerance: f64)
        -> Result<Option<(Self, f64)>> {
        let sector_count = segments.len() / pieces_per_sector;
        let mut pieces = Vec::with_capacity(pieces_per_sector);
        let mut maximum: f64 = 0.;
        for offset in 0..pieces_per_sector {
            let Some((piece, error)) = CircularProfilePattern::compress_column(
                segments, offset, pieces_per_sector, tolerance,
            )? else { return Ok(None); };
            maximum = maximum.max(error);
            pieces.push(piece);
        }
        Ok(Some((Self { pieces, sector_count }, maximum)))
    }

    pub fn sector(&self, index: usize) -> Result<Vec<Curve>> {
        check(index < self.sector_count, "Profile sector index outside pattern")?;
        self.pieces.iter().map(|piece| piece.instance(index).map(|(curve, _)| curve)).collect()
    }
}

/// Lossless representation for ordered copies with identical knot layouts.
/// Rotated seed coordinates predict each copy; byte-packed XOR residuals retain
/// every input coordinate and weight bit, including signed zero.
pub struct LosslessSectorPattern {
    pattern: CircularSectorPattern,
    residuals: Vec<SectorResidual>,
}

struct SectorResidual {
    predictor_index: usize,
    bytes: Vec<u8>,
}

impl LosslessSectorPattern {
    pub fn encode(segments: &[Curve], pieces_per_sector: usize) -> Result<Self> {
        Self::encode_ordered(segments, pieces_per_sector, false)
    }

    /// Compare clockwise and counterclockwise predictors per sector, retaining
    /// the shorter residual stream and preserving the incoming traversal order.
    pub fn encode_bidirectional(segments: &[Curve], pieces_per_sector: usize) -> Result<Self> {
        Self::encode_ordered(segments, pieces_per_sector, true)
    }

    fn encode_ordered(segments: &[Curve], pieces_per_sector: usize, bidirectional: bool) -> Result<Self> {
        check(pieces_per_sector > 0 && !segments.is_empty() && segments.len() % pieces_per_sector == 0,
              "Expected complete profile sectors")?;
        let count = segments.len() / pieces_per_sector;
        let pieces = segments[..pieces_per_sector].iter().map(|c| CircularProfilePattern::new(c.clone(), count))
            .collect::<Result<Vec<_>>>()?;
        let pattern = CircularSectorPattern { pieces, sector_count: count };
        let mut residuals = Vec::with_capacity(count - 1);
        for (i, sector) in segments.chunks_exact(pieces_per_sector).enumerate().skip(1) {
            for actual in sector { actual.validate()?; }
            let mut selected = Self::residual(&pattern, i, sector)?;
            if bidirectional && count - i != i {
                let reverse = Self::residual(&pattern, count - i, sector)?;
                if reverse.bytes.len() < selected.bytes.len() { selected = reverse; }
            }
            residuals.push(selected);
        }
        Ok(Self { pattern, residuals })
    }

    fn residual(pattern: &CircularSectorPattern, predictor_index: usize, sector: &[Curve]) -> Result<SectorResidual> {
        let mut bytes = Vec::new();
        for (actual, predicted) in sector.iter().zip(pattern.sector(predictor_index)?) {
            check(actual.degree == predicted.degree && actual.periodic == predicted.periodic &&
                actual.knots.len() == predicted.knots.len() &&
                actual.knots.iter().zip(&predicted.knots).all(|(a,b)| a.to_bits() == b.to_bits()) &&
                actual.control_points[0].len() == predicted.control_points[0].len(),
                "Lossless sector requires matching spline layout")?;
            let values = actual.control_points.iter().flatten().chain(&actual.weights);
            let predictions = predicted.control_points.iter().flatten().chain(&predicted.weights);
            for (x,y) in values.zip(predictions) {
                let delta = x.to_bits() ^ y.to_bits();
                let size = (64 - delta.leading_zeros()).div_ceil(8) as usize;
                bytes.push(size as u8);
                bytes.extend_from_slice(&delta.to_le_bytes()[..size]);
            }
        }
        Ok(SectorResidual { predictor_index, bytes })
    }

    pub fn residual_bytes(&self) -> usize { self.residuals.iter().map(|s| s.bytes.len()).sum() }

    pub fn sector(&self, index: usize) -> Result<Vec<Curve>> {
        if index == 0 { return self.pattern.sector(0); }
        check(index <= self.residuals.len(), "Profile sector index outside pattern")?;
        let residual = &self.residuals[index-1];
        let mut sector = self.pattern.sector(residual.predictor_index)?;
        let mut bytes = residual.bytes.as_slice();
        for curve in &mut sector {
            for value in curve.control_points.iter_mut().flatten().chain(&mut curve.weights) {
                let size = bytes[0] as usize;
                let mut mask = [0;8];
                mask[..size].copy_from_slice(&bytes[1..1+size]);
                *value = f64::from_bits(value.to_bits() ^ u64::from_le_bytes(mask));
                bytes = &bytes[1+size..];
            }
        }
        Ok(sector)
    }
}
