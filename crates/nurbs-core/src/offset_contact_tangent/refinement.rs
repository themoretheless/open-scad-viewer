//! Nested root-box proposals retain room for outward arithmetic on nearly
//! constant coordinates. No padding is an inclusion or error certificate.
pub(crate) fn padded(root: [f64; 2], input: [f64; 2], original: [f64; 2]) -> [f64; 2] {
    let scale = original[0].abs().max(original[1].abs());
    let root_width = root[1] - root[0];
    let clearance = ((input[1] - input[0]) - root_width).max(0.) * 0.25;
    let margin = (root_width * 0.25)
        .min(clearance)
        .max(scale * (128. * f64::EPSILON))
        .max(f64::from_bits(1));
    // Clamping only the proposal to its last successful input keeps the
    // original unique root. Every attempt is independently requalified. A
    // constant dimension reaching rounding width must not freeze refinement of
    // other dimensions; original source units set the roundoff proposal scale.
    [
        (root[0] - margin).next_down().max(input[0]),
        (root[1] + margin).next_up().min(input[1]),
    ]
}
