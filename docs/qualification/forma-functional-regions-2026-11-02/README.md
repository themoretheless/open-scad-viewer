# Functional Forma web regions

The original five Forma UI tests had four failures. Fixed-data Studio templates
were being used as web application regions, dropping real state, application
slots and dynamic controls. Their original bytes are preserved in src/ui/studio.
The web regions now use App.vue view-model values, expose its existing actions,
and retain file/settings, VR/reference/more, and material/grid slots. GPU and
geometry results come from supplied runtime state rather than prototype numbers.

The UI parser now tokenizes whole comparison/logical operators; evaluation places
unary negation after outer binary/ternary decomposition. Reactivity, cancellation,
checkbox binding, all default primitive/Boolean actions, draft conflict choices
and conversion progress have regressions. All25 selected tests pass. Actual DOM
interactions at1440px and390px pass with no page errors/overflow; images were
visually inspected. These are finite view-model fixtures, not a full application
or a GPU/CAD computation qualification. Types and architecture pass.

Production Vite builds, but verify-dist stops at the absent exact e86 artifact
profile. Read-only measurement also finds three existing delivery limits exceeded:
5,888,240 geometry JS bytes versus5,370,000;16,670,564 raw geometry WASM bytes
versus16,000,000;10,638,879 asset bytes versus10,080,000. No profile or limit was
changed. Native Rust source and both dirty packaged WASM artifacts are preserved.
Original failure runs, including one new assertion that initially read only a
parent node text field instead of its complete text content, remain archived.
