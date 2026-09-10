import type { DirectBody, DirectDocument } from './directModeling'

export function bodyPoints(b: DirectBody): number[][] {
  return Array.from({ length: b.mesh.positions.length / 3 }, (_, i) => b.mesh.positions.slice(i * 3, i * 3 + 3))
}

/** Kernel-free SCAD text for the bodies; kept out of directModeling so history
 * restore does not pull the geometry kernel into the startup graph. */
export function directBodiesScad(document: DirectDocument): string {
  return document.bodies.map(b => {
    // OpenSCAD polyhedron uses clockwise faces, opposite the kernel's outward CCW mesh.
    const faces = Array.from({ length: b.mesh.indices.length / 3 }, (_, i) => b.mesh.indices.slice(i * 3, i * 3 + 3).reverse())
    return `polyhedron(points=${JSON.stringify(bodyPoints(b))},faces=${JSON.stringify(faces)},convexity=10);`
  }).join('\n')
}
