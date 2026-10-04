# Rush in the CAD frontend

The frontend uses `themoretheless-tokenizer-rush` from the tokenizer repository's
`release` branch. `crates/Cargo.lock` pins the revision used by native and WASM
builds. Rebuild the browser artifact with `node scripts/build-language-kernel.mjs`
after updating the dependency.

Use `.r` files and a `// @rush/1` header to select the Rush CAD frontend:

```text
// @rush/1
param radius: 2mm range 1mm..8mm
fn make x: length -> length
  return x
show circle(make(radius)).extrude(4mm)
```

Rush supplies lossless lexical boundaries for comments and strings. The CAD
adapter refines numeric units, ranges and fluent punctuation because the shared
upstream lexer emits broad numeric tokens. `return` maps to the existing return
AST while retaining the original source offsets.

The current CAD parser and ModelGraph execution representation remain in use.
Connecting the tokenizer does not add execution support for every construct
recognized by the upstream Rush editor engine (for example shell commands).
Legacy `// @modelgraph-text/1`, `.mg` files and `ret` remain supported.
