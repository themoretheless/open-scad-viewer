# Native periodic profile embedding predicate

Implementation: `crates/nurbs-core/src/sweeps/profile_geometry.rs`. This predicate is independent of matched-parameter deviation. Its scope is one complete untrimmed surface, with only the V seam identified. Solid topology and pairwise face contacts remain explicitly uncertified.

## Lift and global contraction

For a selected coordinate plane and a constant origin, require the complete represented projection to avoid that origin. The simply connected universal parameter strip U × R then has a continuous unwrapped polar angle. Let F be (squared radius, unwrapped angle), or (orthogonal height, unwrapped angle).

Restrict original homogeneous surface controls to every original knot rectangle. Correlated Bernstein products enclose derivatives of these projected coordinates: the angle numerator is Hx dHy − Hy dHx and denominator Hx² + Hy²; the squared-radius numerator is W d(Hx²+Hy²) − 2(Hx²+Hy²)dW and denominator W³. The height derivative is (dHz W − Hz dW)/W². Positive denominator hulls and outward interval operations are required.

Enclose the entire projected Jacobian, and choose a finite constant floating matrix Y from its midpoint. Bound ||I − Y DF||∞ strictly below one. For any two points in the convex universal strip, integrate this residual along their connecting segment: equality of F at those points would imply their distance is at most q times that distance, so the points coincide. Y need not be an exact numerical inverse; the verified residual is the premise.

## Closing seam and winding

Repeated homogeneous controls and exactly translated periodic knots establish the represented periodic basis identity, including derivative jets. The subtraction checks retain the floating subtraction residual; rounded equality alone is insufficient. The selected chart must have the required C1 basis across interior knots and the closing seam.

A strictly signed angle derivative throughout the complete strip, together with a full-period upper bound below 4π, establishes winding +1 or −1 for its exactly closed nonzero projection. If two surface points coincide, their lifted angles differ by an integer multiple of 2π. Translate one V parameter by the corresponding integer number of periods. The lifted coordinates now coincide, and global contraction proves equality of U and V modulo the seam. A regular double cover therefore cannot pass.

Subdivision always covers original interval endpoints exactly. Work is charged for every attempted chart and cell. Exhaustion returns an unproved predicate, never a partial positive certificate. High source degrees or unsupported chart configurations also return explicit refusal.

## Open spatial frame bound

For uncorrected open Bishop transport, |N′| and |B′| are at most |T′|, and |T′| ≤ |C″|/|C′| in source parameter units. A rigorous whole-prefix jet enclosure with positive speed lower bound therefore bounds each frame component's displacement from its initial interval. Source-domain length is included, so affine parameter changes preserve the geometric bound. If this premise cannot be enclosed, retain the full unit-frame envelope. Closed transport retains that envelope because it includes closing correction.

## Validation boundary

Local native validation at this milestone: 692 unit tests, two integration tests and one doctest passed. Adversarial cases include a regular exact double cover, one-ulp periodic-knot tampering and exhausted work. Open-frame tests compare two source parameter ranges and explicitly exclude closing correction. Additional station counts exercise G1 quadratic and G2 cubic seams.

These checks do not establish arbitrary spatial-frame accuracy, cap embedding, inter-face contacts, shell containment or shell orientation. They do not approve G1 semantic qualification or a production cutover.
