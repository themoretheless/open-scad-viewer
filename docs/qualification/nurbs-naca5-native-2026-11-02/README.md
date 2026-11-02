# Standard nonreflex five-digit NACA profile contract

NURBS-0219 is one geometry family. The encoded builder now uses published mean-line coefficients instead of treating the design-lift digit as percent maximum camber. A typed analytic station evaluator is exposed in the same family; the independently useful custom max-camber constructor is retained.

The original 23012 regression measured mean camber 0.01999846 at x=0.15 against the published nominal equation value 0.01838645. Five public tests now pass, including every supported P row, multiple lift and chord scales, trailing-edge conventions, invalid requests and the old custom solver. All ten selected public suites pass 72 tests, without skips or filters. Library checking without default features passes. Source, Cargo.lock and actual ten test-binary hashes are recorded.

The prior full 2043-test library checkpoint predates this NACA change and is not claimed for this source. Full current library/workspace/application qualification, global G0/G1 and delivery remain separate. Read the design document for supported codes and numerical limitations; primary source references and hashes are recorded in the receipt.

A later full default-library run at source commit e0469e4ab passes 2043 tests with no failures and the same single ignored native microbenchmark. Its separate full-library-check.json, source manifest and log preserve that pre-CST scope.
