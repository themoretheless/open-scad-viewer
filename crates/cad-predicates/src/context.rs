use crate::{IMPLEMENTATION_VERSION, Reason};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

pub const MAX_WORK: u64 = 1_000_000;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn identity() -> Result<u64, InputError> {
    let mut current = NEXT_ID.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_add(1)
            .ok_or(InputError::InvalidInput("Identity space exhausted"))?;
        match NEXT_ID.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(current),
            Err(observed) => current = observed,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
    InvalidInput(&'static str),
    ProvenanceMismatch,
    ContextMismatch,
}
impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for InputError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthoredScalar {
    Binary64Bits(u64),
    RationalConstant { numerator: i64, denominator: u64 },
}

/// A coordinate address into one immutable admitted authored snapshot.
/// It cannot be forged by decoding a caller-supplied provenance tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LeafRef {
    arena: u64,
    index: usize,
}

#[derive(Debug)]
pub struct SourceArena {
    id: u64,
    source_id: String,
    revision: u64,
    values: Vec<AuthoredScalar>,
}
impl SourceArena {
    /// Trusted authored-input boundary. The caller must supply original input
    /// bits, not serialized rounded results from a geometric construction.
    pub fn authored(
        source_id: &str,
        revision: u64,
        values: Vec<AuthoredScalar>,
    ) -> Result<Self, InputError> {
        if source_id.is_empty() || source_id.len() > 256 || values.len() > 65_536 {
            return Err(InputError::InvalidInput(
                "Invalid authored snapshot identity or size",
            ));
        }
        for scalar in &values {
            match *scalar {
                AuthoredScalar::Binary64Bits(bits) if !f64::from_bits(bits).is_finite() => {
                    return Err(InputError::InvalidInput("Authored binary64 must be finite"));
                }
                AuthoredScalar::RationalConstant {
                    numerator,
                    denominator,
                } => {
                    let (mut a, mut b) = (numerator.unsigned_abs(), denominator);
                    while b != 0 {
                        (a, b) = (b, a % b);
                    }
                    if denominator == 0 || a != 1 {
                        return Err(InputError::InvalidInput(
                            "Rational constant must have a positive reduced denominator",
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(Self {
            id: identity()?,
            source_id: source_id.to_owned(),
            revision,
            values,
        })
    }
    pub fn leaf(&self, index: usize) -> Result<LeafRef, InputError> {
        if index >= self.values.len() {
            return Err(InputError::InvalidInput("Unknown source coordinate"));
        }
        Ok(LeafRef {
            arena: self.id,
            index,
        })
    }
    pub fn source_id(&self) -> &str {
        &self.source_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Debug)]
pub struct ToleranceSpec {
    pub linear_abs: f64,
    pub linear_rel: f64,
    pub on_tol: f64,
    pub clear_tol: f64,
    pub angular: f64,
    pub param_floor: f64,
    pub ulp_guard: u32,
    pub max_entity_error: f64,
    pub policy: String,
}

#[derive(Debug)]
pub struct ToleranceContext {
    id: u64,
    pub(crate) spec: ToleranceSpec,
}
impl ToleranceContext {
    pub fn new(spec: ToleranceSpec) -> Result<Self, InputError> {
        if [
            spec.linear_abs,
            spec.on_tol,
            spec.clear_tol,
            spec.angular,
            spec.param_floor,
            spec.max_entity_error,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
            || !spec.linear_rel.is_finite()
            || spec.linear_rel < 0.
            || spec.on_tol >= spec.clear_tol
            || spec.linear_abs > spec.max_entity_error
            || spec.ulp_guard == 0
            || spec.policy.is_empty()
            || spec.policy.len() > 128
        {
            return Err(InputError::InvalidInput("Invalid tolerance context"));
        }
        Ok(Self {
            id: identity()?,
            spec,
        })
    }
    pub fn default_valid() -> Self {
        Self::new(ToleranceSpec {
            linear_abs: 1e-7,
            linear_rel: 1e-12,
            on_tol: 1e-7,
            clear_tol: 1e-5,
            angular: 1e-9,
            param_floor: 1e-12,
            ulp_guard: 4,
            max_entity_error: 1e-6,
            policy: "candidate-plane-box-1".to_owned(),
        })
        .expect("Constant default tolerance is valid")
    }
    pub fn specification(&self) -> &ToleranceSpec {
        &self.spec
    }
}

/// Opaque in-process cache identity binds the full immutable source/tolerance
/// objects and algorithm version. It is deliberately not a serialized certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContextIdentity {
    arena: u64,
    tolerance: u64,
    implementation: &'static str,
}

#[derive(Clone, Debug)]
pub struct Limits {
    pub max_work: u64,
    pub max_expansion_terms: usize,
    pub deadline: Option<Instant>,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_work: MAX_WORK,
            max_expansion_terms: 4096,
            deadline: None,
        }
    }
}

pub struct PredicateContext<'a> {
    arena: &'a SourceArena,
    pub(crate) tolerance: &'a ToleranceContext,
    limits: Limits,
    cancel: Option<&'a AtomicBool>,
    used: u64,
}
impl<'a> PredicateContext<'a> {
    pub fn new(
        arena: &'a SourceArena,
        tolerance: &'a ToleranceContext,
        limits: Limits,
        cancel: Option<&'a AtomicBool>,
    ) -> Self {
        Self {
            arena,
            tolerance,
            limits,
            cancel,
            used: 0,
        }
    }
    pub fn identity(&self) -> ContextIdentity {
        ContextIdentity {
            arena: self.arena.id,
            tolerance: self.tolerance.id,
            implementation: IMPLEMENTATION_VERSION,
        }
    }
    pub(crate) fn source_leaf(&self, index: usize) -> Result<LeafRef, InputError> {
        self.arena.leaf(index)
    }
    pub(crate) fn source_descriptor(&self) -> (&str, u64) {
        (self.arena.source_id(), self.arena.revision())
    }
    pub(crate) fn source_index(&self, leaf: LeafRef) -> Result<usize, InputError> {
        self.resolve(leaf)?;
        Ok(leaf.index)
    }
    pub fn work_used(&self) -> u64 {
        self.used
    }
    pub(crate) fn resolve(&self, reference: LeafRef) -> Result<&AuthoredScalar, InputError> {
        if reference.arena != self.arena.id {
            return Err(InputError::ProvenanceMismatch);
        }
        self.arena
            .values
            .get(reference.index)
            .ok_or(InputError::InvalidInput("Unknown source coordinate"))
    }
    pub(crate) fn charge(&mut self, amount: u64) -> Result<(), Reason> {
        if self.cancel.is_some_and(|v| v.load(Ordering::Relaxed)) {
            return Err(Reason::Cancelled);
        }
        if self.limits.deadline.is_some_and(|v| Instant::now() >= v) {
            return Err(Reason::DeadlineExceeded);
        }
        if self.limits.max_work > MAX_WORK
            || self.limits.max_expansion_terms == 0
            || self.limits.max_expansion_terms > 4096
        {
            return Err(Reason::ResourceLimit);
        }
        let next = self.used.checked_add(amount).ok_or(Reason::ResourceLimit)?;
        if next > self.limits.max_work {
            return Err(Reason::ResourceLimit);
        }
        self.used = next;
        Ok(())
    }
    pub(crate) fn terms(&mut self, count: usize) -> Result<(), Reason> {
        self.charge(0)?;
        if count > self.limits.max_expansion_terms {
            Err(Reason::ResourceLimit)
        } else {
            Ok(())
        }
    }
}
