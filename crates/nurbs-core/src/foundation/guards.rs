//! Unified guard infrastructure strengthening the whole kernel.
//!
//! Checklist items 1065, 1093–1094:
//! - **1065** — one [`Budget`] / [`BudgetGuard`] type for every iterative
//!   procedure (iterations, recursion depth, wall clock). Exhaustion yields
//!   the typed [`BudgetExhausted`] payload carried by a `resource()` error.
//! - **1093** — NaN/Inf boundary validators ([`require_finite_f64`] and
//!   friends) raising typed [`NonFinite`] payloads as `check()` errors, plus
//!   [`assert_finite_debug`] for result control points.
//! - **1094** — canonicalization for hashing: [`canonical_f64`] collapses
//!   `-0.0` into `+0.0` and every NaN bit pattern into one canonical NaN, and
//!   [`hash_f64`] / [`hash_point`] / [`hash_slice`] / [`quantized_hash`] build
//!   FNV-1a digests (same construction as `intersection/isosurface.rs`) over
//!   canonical bits.
//!
//! Existing per-module budgets (`Limits`, `CgOptions`, …) keep their shape;
//! [`Budget::with_iterations`] and the [`IterationBudget`] trait let them feed
//! a unified [`BudgetGuard`] without breaking their APIs.

use crate::{Result, check, input, resource};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Item 1065 — unified budget
// ---------------------------------------------------------------------------

/// Unified budget for iterative procedures: iteration count, recursion
/// depth, and wall-clock time. All three limits are hard; crossing any of
/// them aborts with a `resource()` error carrying [`BudgetExhausted`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Hard cap on `tick()` calls.
    pub max_iterations: usize,
    /// Hard cap on simultaneously open `enter_depth()` scopes.
    pub max_depth: usize,
    /// Hard cap on wall-clock milliseconds since guard creation.
    pub max_wall_clock_millis: u64,
}

impl Budget {
    /// Validated constructor: every limit must be at least 1.
    pub fn new(max_iterations: usize, max_depth: usize, max_wall_clock_millis: u64) -> Result<Self> {
        check(max_iterations >= 1, "Budget needs an iteration limit of at least 1")?;
        check(max_depth >= 1, "Budget needs a depth limit of at least 1")?;
        check(
            max_wall_clock_millis >= 1,
            "Budget needs a wall-clock limit of at least 1 ms",
        )?;
        Ok(Self {
            max_iterations,
            max_depth,
            max_wall_clock_millis,
        })
    }
    /// Budget for a pure iteration loop: no recursion, generous wall clock.
    pub fn with_iterations(max_iterations: usize) -> Result<Self> {
        Self::new(max_iterations, 1, u64::MAX)
    }
    /// Start guarding the stage named `stage` (used in error payloads).
    pub fn guard(&self, stage: &'static str) -> BudgetGuard {
        BudgetGuard {
            budget: *self,
            stage,
            iterations: 0,
            depth: 0,
            start: monotonic_start(self.max_wall_clock_millis),
        }
    }
}

// wasm32-unknown-unknown has no host monotonic clock. Iteration/depth-only
// guards must not call Instant::now (which traps there). A requested finite
// wall-clock limit remains explicitly unsupported, never silently disabled.
fn monotonic_start(wall_millis: u64) -> Option<Instant> {
    if wall_millis == u64::MAX { return None; }
    #[cfg(target_arch = "wasm32")]
    { None }
    #[cfg(not(target_arch = "wasm32"))]
    { Some(Instant::now()) }
}

impl Default for Budget {
    /// 10 000 iterations and depth 64. Native targets also default to a
    /// one-minute wall clock; clockless WASM uses no implicit time limit.
    fn default() -> Self {
        Self {
            max_iterations: 10_000,
            max_depth: 64,
            max_wall_clock_millis: if cfg!(target_arch = "wasm32") { u64::MAX } else { 60_000 },
        }
    }
}

/// Anything with an iteration cap can feed a unified [`Budget`].
pub trait IterationBudget {
    /// Maximum number of iterations the procedure is allowed to perform.
    fn max_iterations(&self) -> usize;
}

impl IterationBudget for Budget {
    fn max_iterations(&self) -> usize {
        self.max_iterations
    }
}

impl IterationBudget for crate::numerics::sparse_linalg::CgOptions {
    fn max_iterations(&self) -> usize {
        self.max_iterations
    }
}

impl<T: IterationBudget> From<&T> for Budget {
    /// Derive a unified budget from any existing options/limits type,
    /// keeping target-appropriate depth and wall-clock defaults.
    fn from(source: &T) -> Self {
        Self {
            max_iterations: source.max_iterations(),
            ..Self::default()
        }
    }
}

/// Typed payload describing which limit of which stage was exhausted.
/// `partial` marks procedures that may hold a usable partial result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetExhausted {
    /// Procedure stage that ran out of budget.
    pub stage: &'static str,
    /// Which limit was crossed.
    pub limit: BudgetLimit,
    /// Whether the procedure may carry a partial result.
    pub partial: bool,
}

/// Which budget dimension was exhausted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetLimit {
    /// `tick()` count crossed `max_iterations`.
    Iterations,
    /// Open depth scopes crossed `max_depth`.
    Depth,
    /// Elapsed wall clock crossed `max_wall_clock_millis`.
    WallClock,
}

impl std::fmt::Display for BudgetExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let limit = match self.limit {
            BudgetLimit::Iterations => "iteration budget",
            BudgetLimit::Depth => "depth budget",
            BudgetLimit::WallClock => "wall-clock budget",
        };
        write!(
            f,
            "{} exhausted its {}{}",
            self.stage,
            limit,
            if self.partial { " (partial result available)" } else { "" },
        )
    }
}

impl BudgetExhausted {
    /// Wrap into the crate `resource()` taxonomy.
    pub fn into_error(self) -> crate::Error {
        resource(self.to_string())
    }
}

impl From<BudgetExhausted> for crate::Error {
    fn from(payload: BudgetExhausted) -> Self {
        payload.into_error()
    }
}

/// RAII-free guard: call [`BudgetGuard::tick`] per iteration,
/// [`BudgetGuard::enter_depth`] / [`BudgetGuard::exit_depth`] around recursive
/// descent, and [`BudgetGuard::check`] at coarse waypoints.
#[derive(Debug)]
pub struct BudgetGuard {
    budget: Budget,
    stage: &'static str,
    iterations: usize,
    depth: usize,
    start: Option<Instant>,
}

impl BudgetGuard {
    /// Count one iteration; error when the iteration budget is spent.
    pub fn tick(&mut self) -> Result<()> {
        self.check()?;
        self.iterations += 1;
        if self.iterations > self.budget.max_iterations {
            return Err(BudgetExhausted {
                stage: self.stage,
                limit: BudgetLimit::Iterations,
                partial: true,
            }
            .into_error());
        }
        Ok(())
    }
    /// Enter one recursion level; error when the depth budget is spent.
    /// Must be paired with [`BudgetGuard::exit_depth`].
    pub fn enter_depth(&mut self) -> Result<()> {
        self.check()?;
        self.depth += 1;
        if self.depth > self.budget.max_depth {
            self.depth -= 1;
            return Err(BudgetExhausted {
                stage: self.stage,
                limit: BudgetLimit::Depth,
                partial: false,
            }
            .into_error());
        }
        Ok(())
    }
    /// Leave the recursion level opened by [`BudgetGuard::enter_depth`].
    pub fn exit_depth(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
    /// Check the wall-clock budget at a coarse waypoint.
    pub fn check(&self) -> Result<()> {
        if self.budget.max_wall_clock_millis == u64::MAX { return Ok(()); }
        let Some(start) = self.start else {
            return Err(resource("Wall-clock budget requires an available host monotonic clock"));
        };
        if start.elapsed().as_millis() > u128::from(self.budget.max_wall_clock_millis) {
            return Err(BudgetExhausted {
                stage: self.stage,
                limit: BudgetLimit::WallClock,
                partial: true,
            }
            .into_error());
        }
        Ok(())
    }
    /// Iterations consumed so far.
    pub fn iterations(&self) -> usize {
        self.iterations
    }
    /// Currently open depth scopes.
    pub fn depth(&self) -> usize {
        self.depth
    }
}

// ---------------------------------------------------------------------------
// Item 1093 — NaN/Inf boundary validators
// ---------------------------------------------------------------------------

/// Typed payload for a non-finite input coordinate. `index` locates the
/// offending element inside the `param` array/slice when applicable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonFinite {
    /// Name of the rejected parameter (e.g. `"knots"`, `"weights"`).
    pub param: &'static str,
    /// Element index inside `param`, when known.
    pub index: Option<usize>,
}

impl std::fmt::Display for NonFinite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.index {
            Some(i) => write!(f, "{}[{}] must be finite (no NaN or Inf)", self.param, i),
            None => write!(f, "{} must be finite (no NaN or Inf)", self.param),
        }
    }
}

impl NonFinite {
    /// Wrap into the crate `check()` taxonomy.
    pub fn into_error(self) -> crate::Error {
        input(self.to_string())
    }
}

impl From<NonFinite> for crate::Error {
    fn from(payload: NonFinite) -> Self {
        payload.into_error()
    }
}

/// Reject NaN and ±Inf on a scalar boundary input.
#[inline]
pub fn require_finite_f64(value: f64, param: &'static str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(NonFinite { param, index: None }.into_error())
    }
}

/// Reject NaN and ±Inf on one indexed element of a named array.
#[inline]
pub fn require_finite_at(value: f64, param: &'static str, index: usize) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(NonFinite {
            param,
            index: Some(index),
        }
        .into_error())
    }
}

/// Reject NaN and ±Inf on every coordinate of a slice.
pub fn require_finite_slice(values: &[f64], param: &'static str) -> Result<()> {
    for (i, &v) in values.iter().enumerate() {
        require_finite_at(v, param, i)?;
    }
    Ok(())
}

/// Reject NaN and ±Inf on a 2D/3D point given as a fixed array.
#[inline]
pub fn require_finite_point<const N: usize>(point: &[f64; N], param: &'static str) -> Result<()> {
    require_finite_slice(point, param)
}

/// Reject NaN and ±Inf on a dynamic-dimension point (`Vec<f64>` control
/// point storage in `core::Curve`).
#[inline]
pub fn require_finite_vec(point: &[f64], param: &'static str) -> Result<()> {
    require_finite_slice(point, param)
}

/// Extension trait adding `.require_finite(param)` to scalar, slice, and
/// point inputs.
pub trait RequireFinite {
    /// Validate that every coordinate is finite.
    fn require_finite(&self, param: &'static str) -> Result<()>;
}

impl RequireFinite for f64 {
    fn require_finite(&self, param: &'static str) -> Result<()> {
        require_finite_f64(*self, param)
    }
}

impl RequireFinite for [f64] {
    fn require_finite(&self, param: &'static str) -> Result<()> {
        require_finite_slice(self, param)
    }
}

impl<const N: usize> RequireFinite for [f64; N] {
    fn require_finite(&self, param: &'static str) -> Result<()> {
        require_finite_slice(self, param)
    }
}

impl RequireFinite for Vec<f64> {
    fn require_finite(&self, param: &'static str) -> Result<()> {
        require_finite_slice(self, param)
    }
}

/// Debug-build assertion plus release-build selective check for control
/// points of computed results. In debug builds this panics on non-finite
/// values; it always returns a `Result` so release builds can opt into the
/// same verification at chosen waypoints.
#[inline]
pub fn assert_finite_debug(value: f64, param: &'static str) -> Result<()> {
    debug_assert!(value.is_finite(), "{param} must be finite, got {value}");
    require_finite_f64(value, param)
}

/// Slice variant of [`assert_finite_debug`].
#[inline]
pub fn assert_finite_slice_debug(values: &[f64], param: &'static str) -> Result<()> {
    debug_assert!(
        values.iter().all(|v| v.is_finite()),
        "{param} must be finite, got {values:?}"
    );
    require_finite_slice(values, param)
}

// ---------------------------------------------------------------------------
// Item 1094 — canonicalization for hashing
// ---------------------------------------------------------------------------

/// Canonical NaN bit pattern (quiet, positive sign, payload 1) — the same
/// shape `f64::NAN` uses on x86-64/aarch64, fixed here so the digest never
/// depends on how the NaN was produced.
pub const CANONICAL_NAN_BITS: u64 = 0x7ff8_0000_0000_0001;

/// Collapse `-0.0` into `+0.0` and every NaN bit pattern into one canonical
/// NaN, so logically equal values hash identically.
#[inline]
pub fn canonical_f64(x: f64) -> f64 {
    if x.is_nan() {
        f64::from_bits(CANONICAL_NAN_BITS)
    } else if x == 0.0 {
        0.0
    } else {
        x
    }
}

/// FNV-1a over byte slices — the same construction used by
/// `intersection/isosurface.rs` for deterministic digests.
#[derive(Clone, Copy, Debug)]
pub struct Fnv1a(u64);

impl Fnv1a {
    /// Offset basis.
    pub const OFFSET: u64 = 0xcbf2_9ce4_8422_325;
    /// Prime.
    pub const PRIME: u64 = 0x0000_0100_0000_01b3;
    /// Fresh hasher at the offset basis.
    #[inline]
    pub fn new() -> Self {
        Self(Self::OFFSET)
    }
    /// Absorb raw bytes.
    #[inline]
    pub fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }
    /// Absorb one canonicalized scalar.
    #[inline]
    pub fn write_f64(&mut self, x: f64) {
        self.write(&canonical_f64(x).to_le_bytes());
    }
    /// Current digest.
    #[inline]
    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a {
    fn default() -> Self {
        Self::new()
    }
}

/// FNV-1a digest of the canonical bits of one scalar.
#[inline]
pub fn hash_f64(x: f64) -> u64 {
    let mut h = Fnv1a::new();
    h.write_f64(x);
    h.finish()
}

/// FNV-1a digest of the canonical bits of one fixed-size point.
#[inline]
pub fn hash_point<const N: usize>(point: &[f64; N]) -> u64 {
    hash_slice(point)
}

/// FNV-1a digest of the canonical bits of a coordinate slice.
pub fn hash_slice(values: &[f64]) -> u64 {
    let mut h = Fnv1a::new();
    for &v in values {
        h.write_f64(v);
    }
    h.finish()
}

/// Quantized digest: `x` is snapped to the grid `round(x / grid) * grid`
/// before canonical hashing, so values within one grid cell share a digest.
/// `grid` is the quantization step — set it to at most the matching
/// tolerance (e.g. `0.1`); it must be finite and positive.
pub fn quantized_hash(x: f64, grid: f64) -> Result<u64> {
    require_finite_f64(x, "quantized_hash value")?;
    check(
        grid.is_finite() && grid > 0.,
        "quantized_hash grid must be finite and positive",
    )?;
    let cell = (x / grid).round();
    // Canonicalize through the grid product so -0.0 cells collapse too.
    Ok(hash_f64(canonical_f64(cell * grid)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_resource(err: &crate::Error) {
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    }
    fn assert_check(err: &crate::Error) {
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    }

    // -- item 1065: budget -------------------------------------------------

    #[test]
    fn budget_iterations_exhausted() {
        let budget = Budget::new(3, 4, 60_000).unwrap();
        let mut guard = budget.guard("test-loop");
        for _ in 0..3 {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_resource(&err);
        assert!(err.contains("test-loop"));
        assert!(err.contains("iteration budget"));
    }

    #[test]
    fn budget_depth_exhausted() {
        let budget = Budget::new(100, 2, 60_000).unwrap();
        let mut guard = budget.guard("recursion");
        guard.enter_depth().unwrap();
        guard.enter_depth().unwrap();
        let err = guard.enter_depth().unwrap_err();
        assert_resource(&err);
        assert!(err.contains("depth budget"));
        // Failed enter rolls back: still at depth 2.
        assert_eq!(guard.depth(), 2);
        guard.exit_depth();
        guard.exit_depth();
        assert_eq!(guard.depth(), 0);
    }

    #[test]
    fn budget_wall_clock_exhausted() {
        // Tiny limit plus a real sleep keeps the test honest and fast.
        let budget = Budget::new(100, 4, 1).unwrap();
        let guard = budget.guard("slow-stage");
        std::thread::sleep(std::time::Duration::from_millis(3));
        let err = guard.check().unwrap_err();
        assert_resource(&err);
        assert!(err.contains("wall-clock budget"));
    }

    #[test]
    fn budget_wall_clock_not_exhausted_immediately() {
        let budget = Budget::new(100, 4, 60_000).unwrap();
        let guard = budget.guard("fast-stage");
        guard.check().unwrap();
    }

    #[test]
    fn budget_rejects_zero_limits() {
        assert!(Budget::new(0, 1, 1).is_err());
        assert!(Budget::new(1, 0, 1).is_err());
        assert!(Budget::new(1, 1, 0).is_err());
    }

    #[test]
    fn budget_from_existing_options() {
        let cg = crate::numerics::sparse_linalg::CgOptions::new(42, 1e-9).unwrap();
        assert_eq!(cg.max_iterations(), 42);
        let budget = Budget::from(&cg);
        assert_eq!(budget.max_iterations, 42);
        assert_eq!(budget.max_depth, Budget::default().max_depth);
    }

    #[test]
    fn budget_exhausted_display_and_partial_flag() {
        let payload = BudgetExhausted {
            stage: "stage",
            limit: BudgetLimit::Iterations,
            partial: true,
        };
        assert!(payload.to_string().contains("partial"));
        assert_eq!(payload.into_error().code, crate::RESOURCE_LIMIT);
    }

    // -- item 1093: non-finite guards --------------------------------------

    #[test]
    fn finite_scalars_pass() {
        require_finite_f64(1.5, "t").unwrap();
        require_finite_f64(-0.0, "t").unwrap();
        2.5.require_finite("t").unwrap();
    }

    #[test]
    fn nan_scalar_rejected_with_param() {
        let err = require_finite_f64(f64::NAN, "weights").unwrap_err();
        assert_check(&err);
        assert!(err.contains("weights"));
    }

    #[test]
    fn inf_scalar_rejected() {
        assert!(require_finite_f64(f64::INFINITY, "t").is_err());
        assert!(require_finite_f64(f64::NEG_INFINITY, "t").is_err());
    }

    #[test]
    fn nan_in_point_reports_param_and_index() {
        let point = [0.0, f64::NAN, 1.0];
        let err = require_finite_point(&point, "control_points").unwrap_err();
        assert_check(&err);
        assert!(err.contains("control_points[1]"), "{err}");
    }

    #[test]
    fn nan_in_vec_reports_index() {
        let point = vec![1.0, 2.0, f64::NAN];
        let err = require_finite_vec(&point, "control_points").unwrap_err();
        assert!(err.contains("control_points[2]"), "{err}");
    }

    #[test]
    fn slice_extension_reports_index() {
        let values = [1.0, f64::INFINITY, 3.0];
        let err = values.require_finite("knots").unwrap_err();
        assert!(err.contains("knots[1]"), "{err}");
    }

    #[test]
    fn assert_finite_debug_accepts_finite() {
        assert_finite_debug(3.0, "result").unwrap();
        assert_finite_slice_debug(&[1.0, 2.0], "result").unwrap();
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "result must be finite")]
    fn assert_finite_debug_panics_on_nan_in_debug() {
        let _ = assert_finite_debug(f64::NAN, "result");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn assert_finite_debug_returns_err_on_nan_in_release() {
        let err = assert_finite_debug(f64::NAN, "result").unwrap_err();
        assert_check(&err);
        assert!(err.contains("result"));
    }

    // -- item 1094: canonical hashing --------------------------------------

    #[test]
    fn canonical_nan_unifies_bit_patterns() {
        let quiet = f64::NAN;
        let payload_a = f64::from_bits(0x7ff8_0000_0000_0007);
        let payload_b = f64::from_bits(0xfff8_0000_0000_0001);
        assert!(payload_a.is_nan() && payload_b.is_nan());
        assert_ne!(payload_a.to_bits(), payload_b.to_bits());
        let c = canonical_f64(quiet);
        assert_eq!(canonical_f64(payload_a).to_bits(), c.to_bits());
        assert_eq!(canonical_f64(payload_b).to_bits(), c.to_bits());
        assert_eq!(c.to_bits(), CANONICAL_NAN_BITS);
    }

    #[test]
    fn negative_zero_hashes_like_positive_zero() {
        assert_eq!(hash_f64(-0.0), hash_f64(0.0));
        assert_eq!(hash_slice(&[-0.0, 1.0]), hash_slice(&[0.0, 1.0]));
    }

    #[test]
    fn distinct_nan_patterns_hash_identically() {
        let a = f64::from_bits(0x7ff8_0000_0000_0007);
        let b = f64::from_bits(0xfff0_0000_0000_0001);
        assert!(a.is_nan() && b.is_nan());
        assert_eq!(hash_f64(a), hash_f64(b));
        assert_eq!(hash_f64(f64::NAN), hash_f64(a));
    }

    #[test]
    fn distinct_values_hash_differently() {
        assert_ne!(hash_f64(1.0), hash_f64(2.0));
        assert_ne!(hash_point(&[0.0, 0.0, 0.0]), hash_point(&[0.0, 0.0, 1.0]));
    }

    #[test]
    fn quantized_hash_same_cell_same_digest() {
        // Both snap to cell 1 (0.12/0.1 = 1.2, 0.14/0.1 = 1.4).
        assert_eq!(quantized_hash(0.12, 0.1).unwrap(), quantized_hash(0.14, 0.1).unwrap());
    }

    #[test]
    fn quantized_hash_different_cell_different_digest() {
        assert_ne!(quantized_hash(0.12, 0.1).unwrap(), quantized_hash(0.30, 0.1).unwrap());
    }

    #[test]
    fn quantized_hash_rejects_bad_inputs() {
        assert!(quantized_hash(f64::NAN, 0.1).is_err());
        assert!(quantized_hash(1.0, 0.0).is_err());
        assert!(quantized_hash(1.0, -0.1).is_err());
        assert!(quantized_hash(1.0, f64::NAN).is_err());
    }

    #[test]
    fn quantized_hash_negative_zero_cell_stable() {
        assert_eq!(
            quantized_hash(-0.01, 0.1).unwrap(),
            quantized_hash(0.01, 0.1).unwrap()
        );
    }

    // -- entry-point integration -------------------------------------------

    #[test]
    fn curve_constructor_rejects_nan_control_point() {
        let points = vec![
            vec![0.0, 0.0],
            vec![f64::NAN, 1.0],
            vec![2.0, 0.0],
        ];
        let err = crate::curve::Curve::from_polyline(points).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("control_points[1]"), "{err}");
    }

    #[test]
    fn curve_constructor_rejects_nan_weight_as_non_finite() {
        let curve = crate::curve::Curve {
            degree: 1,
            knots: vec![0.0, 0.0, 1.0, 1.0],
            control_points: vec![vec![0.0, 0.0], vec![1.0, 1.0]],
            weights: vec![1.0, f64::NAN],
            periodic: false,
        };
        let err = curve.validate().unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        // NaN must surface as a non-finite rejection, not as "weight ≤ 0".
        assert!(err.contains("weights[1]"), "{err}");
        assert!(err.contains("positive, finite"), "{err}");
        assert!(!err.contains("[1e-12, 1e12]"), "{err}");
    }

    #[test]
    fn curve_constructor_rejects_inf_knot() {
        let curve = crate::curve::Curve {
            degree: 1,
            knots: vec![0.0, 0.0, f64::INFINITY, 1.0],
            control_points: vec![vec![0.0, 0.0], vec![1.0, 1.0]],
            weights: vec![1.0, 1.0],
            periodic: false,
        };
        let err = curve.validate().unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("knots[2] must be finite"), "{err}");
    }
}

#[cfg(test)]
mod clockless_budget_tests {
    use super::*;
    #[test]
    fn iteration_budget_does_not_require_a_host_clock() {
        let mut guard=Budget::with_iterations(1).unwrap().guard("clockless");
        assert!(guard.start.is_none());assert!(guard.check().is_ok());
        assert!(guard.tick().is_ok());assert!(guard.tick().is_err());
    }
    #[test]
    fn missing_clock_cannot_silently_accept_a_finite_time_budget() {
        let mut guard=Budget::new(10,2,1).unwrap().guard("finite-time");
        guard.start=None;
        assert!(guard.check().is_err());assert!(guard.tick().is_err());
        assert!(guard.enter_depth().is_err());assert_eq!(guard.iterations(),0);
    }
}
