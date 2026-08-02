# Language compatibility policy

OpenSCAD Viewer independently implements `openscad-viewer-subset@1`. It
supports a documented subset of the OpenSCAD language and is not affiliated
with or endorsed by the OpenSCAD project. Unsupported features produce errors;
the application does not silently delegate to or bundle the official runtime.

The implementation is authored in this repository from public language
behavior. OpenSCAD source code, grammar, tests, diagnostic text, documentation
and bundled libraries must not be copied or mechanically translated. Small
team-authored programs may compare observable behavior against a separately
installed official executable; official binaries are not redistributed by the
browser application.

The official OpenSCAD codebase is GPL-2.0-or-later. Shipping an official or
modified OpenSCAD WebAssembly runtime is therefore a separate distribution and
licensing decision, outside this application's current architecture. See the
[official source license header](https://raw.githubusercontent.com/openscad/openscad/master/src/openscad.cc)
and [COPYING](https://raw.githubusercontent.com/openscad/openscad/master/COPYING).

Language behavior, limits and unsupported features are defined by
`src/core/languageContract.ts`. Observable semantic changes require contract
version/revision review and conformance fixtures.
