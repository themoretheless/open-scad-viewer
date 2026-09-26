//! Generic building-block kernels, embedded at compile time.
//!
//! `WG` anchor convention: each source declares
//! `const WG: u32 = 256;` and `@workgroup_size(WG)`; the runtime may
//! substitute a tuned power-of-two before compilation.

pub const SCALE_ADD_WGSL: &str = include_str!("../shaders/scale_add.wgsl");
pub const ZIP_MUL_WGSL: &str = include_str!("../shaders/zip_mul.wgsl");
pub const BLOCK_SUM_WGSL: &str = include_str!("../shaders/block_sum.wgsl");
/// vec4 variants: 16 bytes streamed per thread instead of 4.
pub const SCALE_ADD4_WGSL: &str = include_str!("../shaders/scale_add4.wgsl");
pub const ZIP_MUL4_WGSL: &str = include_str!("../shaders/zip_mul4.wgsl");

pub const UNARY_WGSL: &str = include_str!("../shaders/unary.wgsl");
pub const AFFINE_WGSL: &str = include_str!("../shaders/affine.wgsl");
pub const BINARY_WGSL: &str = include_str!("../shaders/binary.wgsl");

/// Every shipped kernel (name, source), for validation tests.
pub const ALL: [(&str, &str); 8] = [
    ("unary", UNARY_WGSL),
    ("affine", AFFINE_WGSL),
    ("binary", BINARY_WGSL),
    ("scale_add", SCALE_ADD_WGSL),
    ("zip_mul", ZIP_MUL_WGSL),
    ("block_sum", BLOCK_SUM_WGSL),
    ("scale_add4", SCALE_ADD4_WGSL),
    ("zip_mul4", ZIP_MUL4_WGSL),
];
