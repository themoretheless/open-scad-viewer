# OpenSCAD 3D Viewer

Web-based OpenSCAD editor and viewer using WebGPU for fast, native-quality 3D rendering directly in the browser.

No installation. No server. Just open the page and start modeling.

## Features (Current)

- Live editing of a useful subset of OpenSCAD
- Primitives: cube, sphere, cylinder (with r1/r2)
- Transforms: translate, rotate, scale, mirror, multmatrix
- CSG: union, difference, intersection (visual approximation)
- Color assignment
- Orbit camera, panning, zoom with mouse
- Automatic camera fit
- Grid + axis helpers
- Bilingual interface (Russian / English)
- Dark and light themes
- Persistent code in localStorage
- Four built-in examples

## Quick Start

1. Open the app
2. Edit the code on the left
3. Changes render automatically (or press Ctrl/Cmd + Enter)
4. Drag with left mouse to rotate, right mouse or Shift to pan, wheel to zoom

Supported browsers: recent Chrome, Edge, or Firefox Nightly with WebGPU enabled.

## Examples

Load any of the built-in examples from the toolbar:
- Basic primitives
- CSG operations (difference shown with translucent red)
- Simple house
- Decorative tower

## Current Limitations

The parser implements only a small useful subset of OpenSCAD. Variables, loops, user modules, complex expressions, and many built-ins are not yet supported. CSG operations are visualized rather than producing true boolean geometry.

For accurate results and full language support use the official OpenSCAD application.

## Roadmap & Ideas

Hundreds of improvements are possible. The full list of **200 concrete ideas and suggestions** lives in [architecture.md](architecture.md).

Here are some highlighted directions:

### Language & Parser
- Full expression engine, variables, for-loops, if, user modules and functions
- Real `hull()`, `minkowski()`, `linear_extrude` with twist, `rotate_extrude`
- 2D primitives + `polygon`, `polyhedron`, `text`, `surface`
- `import()` for STL/OBJ and `projection()`
- Proper scoped `let()`, `$fn`/`$fa`/`$fs`, special variables, echo/assert

### Real CSG & Geometry
- Replace visual-only difference with true boolean operations
- Mesh healing, manifold checks, volume/surface calculations
- High-quality normals, vertex welding, better curved tessellation

### Editor Experience
- Replace textarea with Monaco or CodeMirror + full syntax highlighting
- Autocomplete, hover docs, snippets, formatter
- Customizer panel auto-generated from variables
- Multi-file support and project sidebar
- Inline error squiggles with accurate source locations

### Rendering & Visualization
- Shadows, SSAO, multiple lights, PBR materials
- Wireframe + shaded modes, section planes, exploded views
- High-quality transparency
- Measurement tools, view cube, preset cameras, orthographic mode
- Animation timeline and video/GIF export

### Export & Workflow
- Export STL, glTF/GLB, OBJ, 3MF, high-res PNG
- Import existing meshes
- Print preparation helpers (supports preview, bed visualization)
- "Send to slicer" integration

### Architecture & Quality
- Parser and mesh generation in a Web Worker
- Incremental evaluation and caching
- Real test suite (parser + visual regression)
- CI, coverage, accessibility
- Publish reusable parser and web component packages

### Platforms & Ecosystem
- PWA + offline support
- Desktop apps (Tauri)
- VS Code extension
- Shareable links (code in URL)
- Public model gallery
- AI-assisted modeling (prompt → SCAD)

See [architecture.md](architecture.md) for the complete numbered list of 200 ideas covering parser, renderer, UI, testing, distribution, advanced features and blue-sky directions.

## Development

```bash
npm install
npm run dev
```

Build:

```bash
npm run build
npm run preview
```

## Contributing

Ideas, issues, and pull requests are welcome.

Before submitting large changes, consider skimming the 200-item list in architecture.md to avoid duplicating effort and to pick high-impact items.

## License

MIT (or your preferred license — add it here).

## Credits

Built as a compact demonstration of WebGPU + a hand-written OpenSCAD subset parser and evaluator.
