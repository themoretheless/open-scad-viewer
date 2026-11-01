# Native curve and compatible surface extension

NURBS-0215 and NURBS-0216. The curve extension existed previously; this checkpoint fixes multispan extraction, old start-parameter preservation and unrepresentable advance handling, and qualifies the existing tensor-product surface adapter. These are two contracts, not a separate capability per axis, end, dimension or continuity enum.

The old implementation fails four public regressions (one passes); the fixed implementation passes seven curve and three surface tests. The combined set, including the previous 43 contracts, passes 53 tests with no skips or filters. Source and actual test-binary hashes are recorded. Original failing and final passing logs are retained. This is focused native qualification, not full workspace or application delivery.

See the curve and surface design documents for numerical limits and explicit refusal boundaries. The compatibility continuity field echoes a request and is not a G2 certificate.
