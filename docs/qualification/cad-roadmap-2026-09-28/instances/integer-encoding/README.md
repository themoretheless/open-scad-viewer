# Rejected integer encoding optimization

Experiment: replace BigInt/DataView 64-bit writes with two 32-bit number writes. Byte identity held for a 32-body CAD packet and all tested safe signed/unsigned boundary values; a permanent test adds 2000 deterministic safe integer cases and negative zero.

The paired benchmark did not give a consistent performance result:

- First run: median 4.56 → 4.05 ms.
- Repeat: median 9.66 → 10.84 ms.

The optimization was reverted. Runtime still uses the previous BigInt writes and retains the separately qualified bounded field-name cache. Raw samples are preserved here to prevent treating the first run as established acceleration. A faster integer path needs stronger measurements before adoption.
