// @rush/1
// Editable defaults keep units and ranges in the authoring document.
param width = 20mm range 5mm..40mm
param height = 8mm range 2mm..20mm

// Procedural construction is callable from the declarative graph.
fn make width: length, height: length -> Geometry
  return box([width, width, height])

// This identity survives renaming `body` and inserting unrelated parts.
body @id("main-body") = make(width, height)
show body
