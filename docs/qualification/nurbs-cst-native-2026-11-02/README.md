# CST construction and inverse native contracts

NURBS-0220 constructs checked numerical NURBS from CST profiles. NURBS-0221 recovers dimensionless CST coefficients in an inferred or authored world frame and supports explicitly labeled sides. Input/frame variants are counted once.

The original implementation fails two independent round trips: coefficient 0.2 becomes about 0.5 at chord 2.5, and translation by Y=3 makes the lower-side fit fail. Both now pass. Ten public CST tests additionally cover an independent quadratic shape and parabolic NURBS equation, vertical 3D placement, two positive cambered sides, scale-invariant regularization, physical out-of-plane residuals, extreme finite directions, invalid options and unchanged limits. Eleven selected public suites pass all 82 tests without skips or filters.

Source, Cargo.lock and actual eleven test binaries are recorded. Previous full 219-library evidence remains scoped to its recorded pre-CST source; it is not promoted to full CST qualification. Read the design document for numerical and inference limitations. WASM/JSON/Rush/editor/STEP, full workspace, global G0/G1 and main delivery remain unqualified here.
