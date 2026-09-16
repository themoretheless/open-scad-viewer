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

pub const TOLERANCE_SPEC_VERSION: u32 = 1;

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

impl PartialEq for ToleranceSpec {
    fn eq(&self, other: &Self) -> bool {
        self.linear_abs.to_bits() == other.linear_abs.to_bits()
            && self.linear_rel.to_bits() == other.linear_rel.to_bits()
            && self.on_tol.to_bits() == other.on_tol.to_bits()
            && self.clear_tol.to_bits() == other.clear_tol.to_bits()
            && self.angular.to_bits() == other.angular.to_bits()
            && self.param_floor.to_bits() == other.param_floor.to_bits()
            && self.ulp_guard == other.ulp_guard
            && self.max_entity_error.to_bits() == other.max_entity_error.to_bits()
            && self.policy == other.policy
    }
}
impl Eq for ToleranceSpec {}

/// Stable, portable identity of every tolerance-policy bit. Unlike
/// `ContextIdentity`, this survives serialization and process boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToleranceSpecIdentity {
    pub version: u32,
    pub canonical: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialBounds {
    pub absolute_mm: f64,
    pub relative: f64,
    pub on_mm: f64,
    pub clear_mm: f64,
    pub ulp_guard: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngularBounds {
    pub radians: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParametricBounds {
    pub floor: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityErrorBounds {
    pub maximum_mm: f64,
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
    /// Migration profile for legacy B-rep documents that only carry
    /// `toleranceMm`. The scalar remains the positional on-bound; the other
    /// named bounds are deterministic policy, not caller-selected epsilons.
    pub fn from_brep_tolerance_mm(tolerance_mm: f64) -> Result<Self, InputError> {
        Self::new(ToleranceSpec {
            linear_abs: tolerance_mm,
            linear_rel: 1e-12,
            on_tol: tolerance_mm,
            clear_tol: tolerance_mm * 10.,
            angular: 1e-9,
            param_floor: 1e-12,
            ulp_guard: 4,
            max_entity_error: tolerance_mm * 10.,
            policy: "brep-tolerance-v1".to_owned(),
        })
    }
    pub fn specification(&self) -> &ToleranceSpec {
        &self.spec
    }
    pub fn spatial_bounds(&self) -> SpatialBounds {
        SpatialBounds {
            absolute_mm: self.spec.linear_abs,
            relative: self.spec.linear_rel,
            on_mm: self.spec.on_tol,
            clear_mm: self.spec.clear_tol,
            ulp_guard: self.spec.ulp_guard,
        }
    }
    pub fn angular_bounds(&self) -> AngularBounds {
        AngularBounds {
            radians: self.spec.angular,
        }
    }
    pub fn parametric_bounds(&self) -> ParametricBounds {
        ParametricBounds {
            floor: self.spec.param_floor,
        }
    }
    pub fn entity_error_bounds(&self) -> EntityErrorBounds {
        EntityErrorBounds {
            maximum_mm: self.spec.max_entity_error,
        }
    }
    pub fn spec_identity(&self) -> ToleranceSpecIdentity {
        let s = &self.spec;
        let policy = s
            .policy
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        ToleranceSpecIdentity {
            version: TOLERANCE_SPEC_VERSION,
            canonical: format!(
                "{:016x}:{:016x}:{:016x}:{:016x}:{:016x}:{:016x}:{:08x}:{:016x}:{policy}",
                s.linear_abs.to_bits(),
                s.linear_rel.to_bits(),
                s.on_tol.to_bits(),
                s.clear_tol.to_bits(),
                s.angular.to_bits(),
                s.param_floor.to_bits(),
                s.ulp_guard,
                s.max_entity_error.to_bits(),
            ),
        }
    }
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.spec == other.spec
    }
}

impl value_codec::Serialize for ToleranceSpecIdentity {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "version".into(),
            value_codec::Serialize::to_value(&self.version),
        );
        object.insert(
            "canonical".into(),
            value_codec::Serialize::to_value(&self.canonical),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for ToleranceSpecIdentity {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected tolerance identity object"))?
            .clone();
        let version = value_codec::Deserialize::from_value(
            object
                .remove("version")
                .ok_or_else(|| value_codec::error("Missing tolerance identity version"))?,
        )?;
        let canonical = value_codec::Deserialize::from_value(
            object
                .remove("canonical")
                .ok_or_else(|| value_codec::error("Missing canonical tolerance identity"))?,
        )?;
        if version != TOLERANCE_SPEC_VERSION || !object.is_empty() {
            return Err(value_codec::error("Unsupported tolerance identity"));
        }
        Ok(Self { version, canonical })
    }
}

impl value_codec::Serialize for ToleranceSpec {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "version".into(),
            value_codec::Serialize::to_value(&TOLERANCE_SPEC_VERSION),
        );
        object.insert(
            "linearAbsMm".into(),
            value_codec::Serialize::to_value(&self.linear_abs),
        );
        object.insert(
            "linearRelative".into(),
            value_codec::Serialize::to_value(&self.linear_rel),
        );
        object.insert(
            "onMm".into(),
            value_codec::Serialize::to_value(&self.on_tol),
        );
        object.insert(
            "clearMm".into(),
            value_codec::Serialize::to_value(&self.clear_tol),
        );
        object.insert(
            "angularRadians".into(),
            value_codec::Serialize::to_value(&self.angular),
        );
        object.insert(
            "parametricFloor".into(),
            value_codec::Serialize::to_value(&self.param_floor),
        );
        object.insert(
            "ulpGuard".into(),
            value_codec::Serialize::to_value(&self.ulp_guard),
        );
        object.insert(
            "maxEntityErrorMm".into(),
            value_codec::Serialize::to_value(&self.max_entity_error),
        );
        object.insert(
            "policy".into(),
            value_codec::Serialize::to_value(&self.policy),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for ToleranceSpec {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected tolerance specification object"))?
            .clone();
        macro_rules! take {
            ($name:literal) => {
                value_codec::Deserialize::from_value(
                    object
                        .remove($name)
                        .ok_or_else(|| value_codec::error(concat!("Missing field ", $name)))?,
                )?
            };
        }
        let version: u32 = take!("version");
        if version != TOLERANCE_SPEC_VERSION {
            return Err(value_codec::error(
                "Unsupported tolerance specification version",
            ));
        }
        let spec = Self {
            linear_abs: take!("linearAbsMm"),
            linear_rel: take!("linearRelative"),
            on_tol: take!("onMm"),
            clear_tol: take!("clearMm"),
            angular: take!("angularRadians"),
            param_floor: take!("parametricFloor"),
            ulp_guard: take!("ulpGuard"),
            max_entity_error: take!("maxEntityErrorMm"),
            policy: take!("policy"),
        };
        if !object.is_empty() {
            return Err(value_codec::error("Unknown tolerance specification field"));
        }
        ToleranceContext::new(spec.clone())
            .map_err(|_| value_codec::error("Invalid tolerance specification"))?;
        Ok(spec)
    }
}

impl value_codec::Serialize for ToleranceContext {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("spec".into(), value_codec::Serialize::to_value(&self.spec));
        object.insert(
            "identity".into(),
            value_codec::Serialize::to_value(&self.spec_identity()),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for ToleranceContext {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected tolerance context object"))?
            .clone();
        let spec: ToleranceSpec = value_codec::Deserialize::from_value(
            object
                .remove("spec")
                .ok_or_else(|| value_codec::error("Missing tolerance context spec"))?,
        )?;
        let identity: ToleranceSpecIdentity = value_codec::Deserialize::from_value(
            object
                .remove("identity")
                .ok_or_else(|| value_codec::error("Missing tolerance context identity"))?,
        )?;
        if !object.is_empty() {
            return Err(value_codec::error("Unknown tolerance context field"));
        }
        let context = Self::new(spec)
            .map_err(|_| value_codec::error("Invalid tolerance context specification"))?;
        if context.spec_identity() != identity {
            return Err(value_codec::error("Tolerance context identity mismatch"));
        }
        Ok(context)
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
