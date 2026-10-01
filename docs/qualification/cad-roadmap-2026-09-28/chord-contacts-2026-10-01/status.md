# Represented chord contacts and repeated traversals

Source implementation adds exact binary64 orientation fallback, shared endpoint and interior contacts, collinear overlap splitting, directed source occurrence retention, and cancellation of opposite passes before fill classification. NonZero and EvenOdd retain their distinct winding behavior. Retraced spikes and bridges do not add filled boundaries.

Qualification: 212 native unit tests and 1 doctest passed, including 32 chord tests; TypeScript checking passed; 3 diagnostics service tests passed; diff whitespace check passed.

Scope remains represented reconstructed chord geometry. Original rational offset topology is not certified. Multiway coincident proper crossings remain outside qualified support.

The tracked browser WASM is unchanged from commit 7071a049. This source publication does not claim browser execution of the added contact and cancellation cases; rebuilding WASM and browser qualification remain required.
