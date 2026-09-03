# Optional official runtime notices

The files described here are not committed to this repository, fetched by
`npm install`, or included in the browser distribution. They are downloaded
only by the explicit `npm run setup:openscad` command into the gitignored
`.open-scad-runtime/` cache.

## OpenSCAD WebAssembly runtime

- Artifact: `OpenSCAD-2026.09.01-WebAssembly-node.zip`
- Source: <https://files.openscad.org/snapshots/OpenSCAD-2026.09.01-WebAssembly-node.zip>
- Pinned archive SHA-256: `82054dfb4911686de0ee3ea36771dbf81f3d014c3460c8ea069ab4f933f6d888`
- Project source: <https://github.com/openscad/openscad>
- License: GPL-2.0-or-later
- License text: <https://github.com/openscad/openscad/blob/master/COPYING>

The setup command verifies the archive digest and creates a deterministic,
NODERAWFS-disabled locally patched copy for MCP use. That installed runtime
remains GPL-2.0-or-later. Anyone redistributing the generated cache must
independently satisfy the GPL's corresponding-source and license obligations;
the cache must not be added to this application's release artifacts.

## Basic Regular font

- Installed filename: `Basic-Regular.ttf`
- Source revision: [`SorkinType/Basic@202e65ac93bd6977e83b2f10db6b1467e0b348db`](https://github.com/SorkinType/Basic/tree/202e65ac93bd6977e83b2f10db6b1467e0b348db)
- Font URL: <https://raw.githubusercontent.com/SorkinType/Basic/202e65ac93bd6977e83b2f10db6b1467e0b348db/Basic-Regular.ttf>
- Font SHA-256: `f2487f20e2241002d007831ec0e7e9c24ad39e36b49492326c79f3d70fd3b270`
- License: SIL Open Font License 1.1
- Installed license filename: `Basic-OFL.txt`
- License URL: <https://raw.githubusercontent.com/SorkinType/Basic/202e65ac93bd6977e83b2f10db6b1467e0b348db/OFL.txt>
- License SHA-256: `25be5240815dc880cfad3606b03c9f05125252e9ce542f733cbbf87837ac810f`

The setup command verifies the font and license before binding both into the
runtime manifest. The runner copies the verified font into each request's MEMFS
and supplies a minimal Fontconfig configuration, so `text()` works without
ambient system-font access. Project-supplied fonts may be added under the
bounded virtual `fonts/` directory. Any redistribution of the font must retain
the OFL text and satisfy that license's conditions.
