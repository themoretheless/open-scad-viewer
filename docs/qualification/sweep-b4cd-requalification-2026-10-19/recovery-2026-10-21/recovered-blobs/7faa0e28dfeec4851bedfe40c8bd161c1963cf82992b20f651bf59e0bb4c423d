import type { DirectBody, DirectDocument } from './directModeling'

export function bodyPoints(b: DirectBody): number[][] {
  const pos = b.mesh.positions
  const count = pos.length / 3
  const out = new Array<number[]>(count)
  for (let i = 0, j = 0; i < count; i++, j += 3) {
    out[i] = [pos[j], pos[j + 1], pos[j + 2]]
  }
  return out
}

/** Kernel-free SCAD text for the bodies; kept out of directModeling so history
 * restore does not pull the geometry kernel into the startup graph. */
export function directBodiesScad(document: DirectDocument): string {
  return document.bodies.map(b => {
    // OpenSCAD polyhedron uses clockwise faces, opposite the kernel's outward CCW mesh.
    const idx = b.mesh.indices
    const faceCount = idx.length / 3
    const faces = new Array<[number, number, number]>(faceCount)
    for (let i = 0, j = 0; i < faceCount; i++, j += 3) {
      faces[i] = [idx[j + 2], idx[j + 1], idx[j]]
    }
    return `polyhedron(points=${JSON.stringify(bodyPoints(b))},faces=${JSON.stringify(faces)},convexity=10);`
  }).join('\n')
}
