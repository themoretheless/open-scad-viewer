# Solid → Blender snapshot bridge

In Solid, choose **File → Export for Blender** (also available in the command palette). The project ID is assigned once, saved in the JSON document, and retained across Undo/Redo. Re-export after editing and import the new snapshot into Blender.

Alternatively, export the current bodies of a saved Solid JSON document from the CLI:

```sh
node --import tsx scripts/export-blender-snapshot.mts model.json model.osv-blender.json my-project
```

Keep the same explicit project ID for every update of that project. Use a different ID for a different model. Body IDs come from the Solid document; renaming files does not change identity.

Install `solid_bridge.py` as a Blender Python add-on, enable it, and use **File → Import → Solid Snapshot**. Reimport the exported file to update the existing objects. The file operator supports Blender Undo. Tested in a separate factory scene using Blender 5.3.0 Alpha, build `870477355736`; other versions are not yet qualified.

## Supported behavior

- Exports current tessellated body geometry, including resolved linked instances.
- New objects receive the source color, metallic and roughness in a Principled BSDF material. Hex colors are converted from sRGB to linear RGB. Subsequent updates preserve the Blender material, including user edits, by default. Enable **Update materials from Solid** in the import options to replace the imported objects’ material slots with the current source material (or clear slots when the source has no material). Shared materials on unrelated objects are not modified. Replaced material datablocks are retained in Blender.
- New objects use scale 0.001 divided by the Blender scene unit scale, preserving millimetres in both metre and centimetre scenes. Existing object transforms are preserved on updates, including user scale overrides.
- Matches objects by project/body ID, preserving their identity, names, transforms, modifiers, object properties and a single Blender material slot.
- Objects absent from a later snapshot remain in the scene with `osv_missing_from_source = True`. Reappearing IDs update those same objects.
- Unrelated objects remain untouched. Meshes shared with other objects are replaced only on the bridge object.
- Validates the complete payload and stages replacement meshes before applying updates. Unexpected errors during the subsequent Blender mutation phase are not transactionally rolled back.

## Limits

This is a body-only mesh exchange. Sketches, curves and surfaces cause an explicit export error. STEP/NURBS, live synchronization, linked Blender mesh instances, C4D and Sub-D reconstruction are not implemented here.

Object animation is preserved: qualified location keyframes at frames 1 and 10 still evaluate to their original values after a geometry update. Mesh-data animation is refused because its topology-dependent channels cannot be safely remapped.

Updates refuse vertex groups, shape keys, UVs, color/custom attributes, mesh custom properties, custom normals, smooth shading, and multiple material slots. Make an independent copy for those Blender edits. Modifiers remain attached, but topology-dependent modifier results are not guaranteed after source topology changes. Local meshes in Object Mode are required. Payload limits: 1,200 bodies, 50,000 vertices and 50,000 triangles per body; file operator limit 64 MB.

Mesh construction uses [`Mesh.from_pydata` and `Mesh.validate`](https://docs.blender.org/api/5.3/bpy.types.Mesh.html). If Blender repairs invalid geometry, the update is refused.

## Verification

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python-exit-code 1 --python scripts/verify-blender-bridge.py
```

The check covers repeat update, identity, transforms, modifiers, material, unrelated objects, invalid input, missing/reappearing bodies, UV refusal and registered file import. TypeScript tests separately cover export of current instance geometry and refusal of incomplete exports.

### CAD export → Blender update qualification

```sh
node --import tsx scripts/export-blender-qualification.mts /tmp/solid-blender-qualification
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python-exit-code 1 --python scripts/verify-blender-roundtrip.py -- /tmp/solid-blender-qualification
```

This exports a native CAD box and its translated instance twice, changing width from 2 to 7 mm. The Blender file operator checks both updated bounds, millimetre-to-metre placement, preserved object identities, user transform/name/modifier/material and an unrelated object. It writes `verification.json`. This qualifies the mesh snapshot path, not reverse Blender-to-CAD geometry conversion.

The first roundtrip run also saves a dedicated `bridge-scene.blend`. Check persistence in a **second Blender process**:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python-exit-code 1 --python scripts/verify-blender-roundtrip.py -- /tmp/solid-blender-qualification --resume
```

The resumed check verifies saved names, transforms, modifier, material, parent and custom object property before updating. It imports both geometry versions again and confirms exactly two bridge objects remain. Pointer equality is tested within each process; cross-process identity is established through persisted project/body IDs. Results are written to `verification-reopened.json`.


### Real browser downloads

After building the app and generating qualification fixtures:

```sh
node scripts/check-solid-download-browser.mjs /tmp/solid-blender-qualification
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python-exit-code 1 --python scripts/verify-blender-roundtrip.py -- /tmp/solid-blender-qualification --browser-downloads
```

The browser check uses the repository's isolated Playwright qualification package. It drives the Solid file menu, saves actual downloads to disk, verifies changed dimensions and stable project identity, and repeats export after reload. The Blender command consumes those downloaded snapshots, preserving the same independent geometry/state checks as the CLI fixture path. Qualified browser: Chromium 151.0.7922.34.

Opening a Solid JSON file without `blenderProjectId` creates a separate exchange identity. To update the same Blender project from later file versions, retain that field when saving the Solid project. Ordinary edits preserve identity; Undo/Redo across file imports restore the corresponding project's identity.
