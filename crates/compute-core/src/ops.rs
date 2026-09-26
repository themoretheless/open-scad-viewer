/// Elementwise f32 arithmetic. Follows WGSL floating-point semantics; portable
/// results are intended for finite inputs with nonzero divisors. Reduction
/// order differs from a sequential CPU sum, so compare with a suitable tolerance.
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug)]
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
