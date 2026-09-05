---
name: modelgraph
description: Build and revise parameterized 3D models using the ModelGraph MCP language, validate real geometry and export through the connected OpenSCAD tools.
---

Read the MCP resource openscad://language/modelgraph-1 before authoring documents. It contains the current JSON Schema, functional language guide, units and examples. If resource reading is unavailable, read references/modelgraph-1-prompt.md and the accompanying schema in this skill.

Use ModelGraph JSON as the source of truth. Prefer strict units, named parameters and pure reusable functions. Run modelgraph_compile, then modelgraph_check. Fix structured errors and inspect actual geometry measurements. Use modelgraph_set_parameters with the returned document hash for parameter edits. Keep the returned document in the user's project when they ask to save work.

When available, use modelgraph_export with the document for STL, stl_binary, 3MF, OBJ, PLY, OFF or AMF. Otherwise use the generated source with openscad_export. Inspect available tool schemas before calling them; read the returned artifact resource. This integration uses an ephemeral catalog per process: save desired documents and export artifacts before the session ends. Do not promise persistence in the MCP catalog.

Report separately: schema validation, declared constraints, actual geometry checks and printability checks. Declared wall thickness constraints do not measure mesh wall thickness. Call modelgraph_report for front/top/isometric images and operation provenance; inspect images_status because bounded previews can be unavailable. Never claim to have visually inspected an image or tested a print unless that actually happened. Unsupported operations should be reported explicitly, not fabricated.

For assemblies, use named anchors and optional limited revolute/slider joints. Read the language resource for nested assembly semantics. Call modelgraph_interference to check current-pose volume overlap (up to 8 leaf components); unknown does not mean clear. This does not check contact or swept motion.

For NURBS, read modelgraph_nurbs_language or openscad://language/modelgraph-nurbs-1 and use explicit modelgraph/nurbs-1 documents. This is our own TypeScript spline and tessellation kernel. Call compile/evaluate/build/export tools with the modelgraph_nurbs_ prefix. JSON retains native rational definitions; STL/3MF/AMF require a derived closed mesh; OBJ/PLY/OFF can represent open meshes. Do not claim STEP, general B-rep booleans or certified printability.
