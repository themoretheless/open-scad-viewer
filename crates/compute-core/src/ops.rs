/// Elementwise f32 arithmetic. Follows WGSL floating-point semantics; portable
/// results are intended for finite inputs with nonzero divisors. Reduction
/// order differs from a sequential CPU sum, so compare with a suitable tolerance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Min,
    Max,
}

/// Elementwise WGSL math. Callers must respect each function's domain:
/// sqrt requires nonnegative inputs, log positive inputs, reciprocal nonzero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum UnaryOp {
    Negate,
    Abs,
    Square,
    Sqrt,
    Reciprocal,
    Exp,
    Log,
    Sin,
    Cos,
}

/// Elementwise f32 comparisons yielding a u32 zero/one mask. Portable results
/// are defined for finite inputs, following WGSL comparison semantics; no
/// cross-backend contract is promised for NaN or infinity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
