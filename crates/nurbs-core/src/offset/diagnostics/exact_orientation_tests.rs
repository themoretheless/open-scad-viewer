use super::*;
#[test]
fn diagonal_collinearity_and_nearby_sides_are_distinct() {
    let a = [1e9, 1e9];
    let b = [1e9 + 4., 1e9 + 6.];
    let c = [1e9 + 2., 1e9 + 3.];
    assert_eq!(orientation(a, b, c).unwrap(), Some(0));
    assert_eq!(
        orientation(a, b, [c[0], f64::from_bits(c[1].to_bits() + 1)]).unwrap(),
        Some(1)
    );
    assert_eq!(
        orientation(a, b, [c[0], f64::from_bits(c[1].to_bits() - 1)]).unwrap(),
        Some(-1)
    );
    assert_eq!(
        orientation([1.25, 3.5], [2.5, 7.], [3.75, 10.5]).unwrap(),
        Some(0)
    );
}
