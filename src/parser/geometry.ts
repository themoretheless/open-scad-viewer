/**
 * Pure mesh-generation geometry: primitive builders (makeCube, makeSphere,
 * makeGear, …), 2D triangulation (earClip), extrusion, and 3D convex hull.
 *
 * Extracted from the openscadParser monolith. Everything here is a pure
 * function of its inputs — no parser/evaluator state — so it can be tested
 * in isolation and later moved to a Web Worker wholesale.
 */
import type { Vec3 } from '../services/math3d'
import type { MeshData } from '../core/mesh'
import { MAX_FN } from './limits'

const BITMAP_FONT: Record<string, number[]> = {
  ' ': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '!': [0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,0,0, 0,0,1,0,0],
  '"': [0,1,0,1,0, 0,1,0,1,0, 0,1,0,1,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '#': [0,1,0,1,0, 1,1,1,1,1, 0,1,0,1,0, 0,1,0,1,0, 0,1,0,1,0, 1,1,1,1,1, 0,1,0,1,0],
  '$': [0,0,1,0,0, 0,1,1,1,1, 1,0,1,0,0, 0,1,1,1,0, 0,0,1,0,1, 1,1,1,1,0, 0,0,1,0,0],
  '%': [1,1,0,0,1, 1,1,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,0,0,0, 0,1,0,1,1, 1,0,0,1,1],
  '&': [0,1,1,0,0, 1,0,0,1,0, 1,0,1,0,0, 0,1,0,0,0, 1,0,1,0,1, 1,0,0,1,0, 0,1,1,0,1],
  "'": [0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '(': [0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0],
  ')': [0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0],
  '*': [0,0,0,0,0, 0,0,1,0,0, 1,0,1,0,1, 0,1,1,1,0, 1,0,1,0,1, 0,0,1,0,0, 0,0,0,0,0],
  '+': [0,0,0,0,0, 0,0,1,0,0, 0,0,1,0,0, 1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,0,0],
  ',': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,1,0,0, 0,1,0,0,0],
  '-': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '.': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0],
  '/': [0,0,0,0,1, 0,0,0,1,0, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 0,1,0,0,0, 1,0,0,0,0],
  '0': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,1,1, 1,0,1,0,1, 1,1,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '1': [0,0,1,0,0, 0,1,1,0,0, 1,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 1,1,1,1,1],
  '2': [0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,1,1,1,1],
  '3': [0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,1,1,0, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '4': [0,0,0,1,0, 0,0,1,1,0, 0,1,0,1,0, 1,0,0,1,0, 1,1,1,1,1, 0,0,0,1,0, 0,0,0,1,0],
  '5': [1,1,1,1,1, 1,0,0,0,0, 1,1,1,1,0, 0,0,0,0,1, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '6': [0,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '7': [1,1,1,1,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  '8': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '9': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,0,0,0,1, 0,1,1,1,0],
  ':': [0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0, 0,0,0,0,0],
  ';': [0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,0,1,0,0, 0,1,0,0,0],
  '<': [0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,0,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0],
  '=': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,0,0, 0,0,0,0,0],
  '>': [0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0],
  '?': [0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,0,0,0,0, 0,0,1,0,0],
  '@': [0,1,1,1,0, 1,0,0,0,1, 1,0,1,1,1, 1,0,1,0,1, 1,0,1,1,1, 1,0,0,0,0, 0,1,1,1,0],
  'A': [0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1],
  'B': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0],
  'C': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,1, 0,1,1,1,0],
  'D': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0],
  'E': [1,1,1,1,1, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,1],
  'F': [1,1,1,1,1, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0],
  'G': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,0, 1,0,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'H': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'I': [1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 1,1,1,1,1],
  'J': [0,0,1,1,1, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 1,0,0,1,0, 0,1,1,0,0],
  'K': [1,0,0,0,1, 1,0,0,1,0, 1,0,1,0,0, 1,1,0,0,0, 1,0,1,0,0, 1,0,0,1,0, 1,0,0,0,1],
  'L': [1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,1],
  'M': [1,0,0,0,1, 1,1,0,1,1, 1,0,1,0,1, 1,0,1,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'N': [1,0,0,0,1, 1,1,0,0,1, 1,0,1,0,1, 1,0,0,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'O': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'P': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0],
  'Q': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,1,0,1, 1,0,0,1,0, 0,1,1,0,1],
  'R': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,1,0,0, 1,0,0,1,0, 1,0,0,0,1],
  'S': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,0, 0,1,1,1,0, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'T': [1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  'U': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'V': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,1,0,1,0, 0,0,1,0,0],
  'W': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,1,0,1, 1,0,1,0,1, 1,1,0,1,1, 1,0,0,0,1],
  'X': [1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1, 1,0,0,0,1],
  'Y': [1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  'Z': [1,1,1,1,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,0,0,0,0, 1,1,1,1,1],
  '[': [0,1,1,1,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,1,1,0],
  '\\': [1,0,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,0,1],
  ']': [0,1,1,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,1,1,1,0],
  '^': [0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '_': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1],
  '`': [0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  'a': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 0,0,0,0,1, 0,1,1,1,1, 1,0,0,0,1, 0,1,1,1,1],
  'b': [1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0],
  'c': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,1, 0,1,1,1,0],
  'd': [0,0,0,0,1, 0,0,0,0,1, 0,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1],
  'e': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,0, 0,1,1,1,0],
  'f': [0,0,1,1,0, 0,1,0,0,1, 0,1,0,0,0, 1,1,1,1,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0],
  'g': [0,0,0,0,0, 0,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,1,1,1,0],
  'h': [1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'i': [0,0,1,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,1,1,0],
  'j': [0,0,0,1,0, 0,0,0,0,0, 0,0,1,1,0, 0,0,0,1,0, 0,0,0,1,0, 1,0,0,1,0, 0,1,1,0,0],
  'k': [1,0,0,0,0, 1,0,0,0,0, 1,0,0,1,0, 1,0,1,0,0, 1,1,0,0,0, 1,0,1,0,0, 1,0,0,1,0],
  'l': [0,1,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,1,1,0],
  'm': [0,0,0,0,0, 0,0,0,0,0, 1,1,0,1,0, 1,0,1,0,1, 1,0,1,0,1, 1,0,1,0,1, 1,0,0,0,1],
  'n': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'o': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'p': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0],
  'q': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,0,0,0,1],
  'r': [0,0,0,0,0, 0,0,0,0,0, 1,0,1,1,0, 1,1,0,0,1, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0],
  's': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,1, 1,0,0,0,0, 0,1,1,1,0, 0,0,0,0,1, 1,1,1,1,0],
  't': [0,1,0,0,0, 0,1,0,0,0, 1,1,1,1,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,1, 0,0,1,1,0],
  'u': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,1,1, 0,1,1,0,1],
  'v': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0],
  'w': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,1,0,1, 1,0,1,0,1, 1,0,1,0,1, 0,1,0,1,0],
  'x': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1],
  'y': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,1,1,1,0],
  'z': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,1,1,1,1],
  '{': [0,0,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,1,0],
  '|': [0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  '}': [0,1,0,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,0,0,0],
  '~': [0,0,0,0,0, 0,0,0,0,0, 0,1,0,0,0, 1,0,1,0,1, 0,0,0,1,0, 0,0,0,0,0, 0,0,0,0,0],
}

export function makeText(text: string, size: number, spacing: number): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  const pixelSize = size / 7
  let vertexOffset = 0
  for (let ci = 0; ci < text.length; ci++) {
    const ch = text[ci]
    const bitmap = BITMAP_FONT[ch]
    if (!bitmap) continue
    const xOff = ci * (5 + spacing) * pixelSize
    for (let row = 0; row < 7; row++) {
      for (let col = 0; col < 5; col++) {
        if (bitmap[row * 5 + col] === 0) continue
        // Create a small cube for this pixel
        const px = xOff + col * pixelSize
        const py = (6 - row) * pixelSize // flip Y so top row is highest
        const pz = 0
        const ps = pixelSize
        // 6 faces, 4 verts each, 6 floats per vert (pos + normal)
        const faces: [number[],number[],number[],number[],number[]][] = [
          [[px,py,pz+ps],[px+ps,py,pz+ps],[px+ps,py+ps,pz+ps],[px,py+ps,pz+ps],[0,0,1]],
          [[px+ps,py,pz],[px,py,pz],[px,py+ps,pz],[px+ps,py+ps,pz],[0,0,-1]],
          [[px,py+ps,pz],[px,py+ps,pz+ps],[px+ps,py+ps,pz+ps],[px+ps,py+ps,pz],[0,1,0]],
          [[px,py,pz+ps],[px,py,pz],[px+ps,py,pz],[px+ps,py,pz+ps],[0,-1,0]],
          [[px+ps,py,pz+ps],[px+ps,py,pz],[px+ps,py+ps,pz],[px+ps,py+ps,pz+ps],[1,0,0]],
          [[px,py,pz],[px,py,pz+ps],[px,py+ps,pz+ps],[px,py+ps,pz],[-1,0,0]],
        ]
        for (const [a, b, c, d, n] of faces) {
          v.push(a[0],a[1],a[2],n[0],n[1],n[2])
          v.push(b[0],b[1],b[2],n[0],n[1],n[2])
          v.push(c[0],c[1],c[2],n[0],n[1],n[2])
          v.push(d[0],d[1],d[2],n[0],n[1],n[2])
          ix.push(vertexOffset, vertexOffset+1, vertexOffset+2, vertexOffset, vertexOffset+2, vertexOffset+3)
          vertexOffset += 4
        }
      }
    }
  }
  return { v, ix }
}

/* ── Mesh generators ──────────────────────────────── */

export function makeCube(sx: number, sy: number, sz: number, center: boolean) {
  const x0 = center ? -sx / 2 : 0, x1 = center ? sx / 2 : sx
  const y0 = center ? -sy / 2 : 0, y1 = center ? sy / 2 : sy
  const z0 = center ? -sz / 2 : 0, z1 = center ? sz / 2 : sz
  const faces: [Vec3, Vec3, Vec3, Vec3, Vec3][] = [
    [[x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1],[0,0,1]],
    [[x1,y0,z0],[x0,y0,z0],[x0,y1,z0],[x1,y1,z0],[0,0,-1]],
    [[x0,y1,z0],[x0,y1,z1],[x1,y1,z1],[x1,y1,z0],[0,1,0]],
    [[x0,y0,z1],[x0,y0,z0],[x1,y0,z0],[x1,y0,z1],[0,-1,0]],
    [[x1,y0,z1],[x1,y0,z0],[x1,y1,z0],[x1,y1,z1],[1,0,0]],
    [[x0,y0,z0],[x0,y0,z1],[x0,y1,z1],[x0,y1,z0],[-1,0,0]],
  ]
  const v: number[] = [], ix: number[] = []
  let vi = 0
  for (const [a, b, c, d, n] of faces) {
    v.push(a[0],a[1],a[2],n[0],n[1],n[2], b[0],b[1],b[2],n[0],n[1],n[2],
           c[0],c[1],c[2],n[0],n[1],n[2], d[0],d[1],d[2],n[0],n[1],n[2])
    ix.push(vi,vi+1,vi+2, vi,vi+2,vi+3); vi += 4
  }
  return { v, ix }
}

export function makeSphere(r: number, seg: number) {
  const v: number[] = [], ix: number[] = []
  for (let ri = 0; ri <= seg; ri++) {
    const phi = Math.PI * ri / seg, sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= seg; si++) {
      const th = 2 * Math.PI * si / seg
      const nx = sp * Math.cos(th), ny = cp, nz = sp * Math.sin(th)
      v.push(r * nx, r * ny, r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < seg; ri++)
    for (let si = 0; si < seg; si++) {
      const a = ri * (seg + 1) + si, b = a + seg + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  return { v, ix }
}

export function makeCylinder(h: number, r1: number, r2: number, center: boolean, fn: number) {
  const v: number[] = [], ix: number[] = []
  const z0 = center ? -h / 2 : 0, z1 = center ? h / 2 : h
  const slopeLen = Math.sqrt(h * h + (r1 - r2) ** 2)
  const nzS = slopeLen > 0 ? (r1 - r2) / slopeLen : 0
  const nrS = slopeLen > 0 ? h / slopeLen : 1
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z0, ca*nrS, sa*nrS, nzS)
    v.push(r2*ca, r2*sa, z1, ca*nrS, sa*nrS, nzS)
  }
  for (let i = 0; i < fn; i++) {
    const a = i * 2; ix.push(a, a+1, a+2, a+2, a+1, a+3)
  }
  const bi = v.length / 6
  v.push(0, 0, z0, 0, 0, -1)
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn
    v.push(r1 * Math.cos(a), r1 * Math.sin(a), z0, 0, 0, -1)
  }
  for (let i = 0; i < fn; i++) ix.push(bi, bi+i+2, bi+i+1)
  const ti = v.length / 6
  v.push(0, 0, z1, 0, 0, 1)
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn
    v.push(r2 * Math.cos(a), r2 * Math.sin(a), z1, 0, 0, 1)
  }
  for (let i = 0; i < fn; i++) ix.push(ti, ti+i+1, ti+i+2)
  return { v, ix }
}

export function makePipe(h: number, r1: number, r2: number, center: boolean, fn: number) {
  const v: number[] = [], ix: number[] = []
  const z0 = center ? -h / 2 : 0, z1 = center ? h / 2 : h

  // Outer wall
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z0, ca, sa, 0)
    v.push(r1*ca, r1*sa, z1, ca, sa, 0)
  }
  for (let i = 0; i < fn; i++) {
    const a = i * 2; ix.push(a, a+1, a+2, a+2, a+1, a+3)
  }

  // Inner wall (normals point inward)
  const innerBase = v.length / 6
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r2*ca, r2*sa, z0, -ca, -sa, 0)
    v.push(r2*ca, r2*sa, z1, -ca, -sa, 0)
  }
  for (let i = 0; i < fn; i++) {
    const a = innerBase + i * 2; ix.push(a, a+2, a+1, a+1, a+2, a+3)
  }

  // Bottom ring cap (z0, normal pointing down)
  const botBase = v.length / 6
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z0, 0, 0, -1)
    v.push(r2*ca, r2*sa, z0, 0, 0, -1)
  }
  for (let i = 0; i < fn; i++) {
    const a = botBase + i * 2
    ix.push(a, a+2, a+1, a+1, a+2, a+3)
  }

  // Top ring cap (z1, normal pointing up)
  const topBase = v.length / 6
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z1, 0, 0, 1)
    v.push(r2*ca, r2*sa, z1, 0, 0, 1)
  }
  for (let i = 0; i < fn; i++) {
    const a = topBase + i * 2
    ix.push(a, a+1, a+2, a+2, a+1, a+3)
  }

  return { v, ix }
}

export function makeWedge(sx: number, sy: number, sz: number) {
  // Wedge: triangular prism along Y axis
  // Bottom face is rectangle at z=0, top edge at z=sz
  // Vertices:
  //   0: (0,0,0)  1: (sx,0,0)  2: (sx,sy,0)  3: (0,sy,0)  -- bottom rectangle
  //   4: (0,0,sz) 5: (0,sy,sz) -- top edge (x=0 side)
  const v: number[] = []
  const ix: number[] = []

  // Front face (y=0): triangle 0,1,4
  const nf = [0, -1, 0]
  v.push(0,0,0, nf[0],nf[1],nf[2])
  v.push(sx,0,0, nf[0],nf[1],nf[2])
  v.push(0,0,sz, nf[0],nf[1],nf[2])
  ix.push(0,1,2)

  // Back face (y=sy): triangle 3,5,2
  const nb = [0, 1, 0]
  const bi = v.length / 6
  v.push(0,sy,0, nb[0],nb[1],nb[2])
  v.push(0,sy,sz, nb[0],nb[1],nb[2])
  v.push(sx,sy,0, nb[0],nb[1],nb[2])
  ix.push(bi, bi+1, bi+2)

  // Bottom face (z=0): rectangle 0,3,2,1
  const nd = [0, 0, -1]
  const di = v.length / 6
  v.push(0,0,0, nd[0],nd[1],nd[2])
  v.push(0,sy,0, nd[0],nd[1],nd[2])
  v.push(sx,sy,0, nd[0],nd[1],nd[2])
  v.push(sx,0,0, nd[0],nd[1],nd[2])
  ix.push(di, di+1, di+2, di, di+2, di+3)

  // Left face (x=0): rectangle 0,4,5,3
  const nl = [-1, 0, 0]
  const li = v.length / 6
  v.push(0,0,0, nl[0],nl[1],nl[2])
  v.push(0,0,sz, nl[0],nl[1],nl[2])
  v.push(0,sy,sz, nl[0],nl[1],nl[2])
  v.push(0,sy,0, nl[0],nl[1],nl[2])
  ix.push(li, li+1, li+2, li, li+2, li+3)

  // Slope face: from (sx,0,0)-(sx,sy,0) up to (0,0,sz)-(0,sy,sz)
  // Normal: cross product of edges
  // The slope connects: (sx,0,0), (sx,sy,0), (0,sy,sz), (0,0,sz)
  // Normal = normalize(cross((sx,sy,0)-(sx,0,0), (0,0,sz)-(sx,0,0)))
  //        = normalize(cross((0,sy,0), (-sx,0,sz)))
  //        = (sy*sz, 0, sy*sx) -> normalize -> (sz, 0, sx) / len
  const sn = Math.sqrt(sz*sz + sx*sx)
  const nsSlope = sn > 0 ? [sz/sn, 0, sx/sn] : [0, 0, 1]
  const si2 = v.length / 6
  v.push(sx,0,0, nsSlope[0],nsSlope[1],nsSlope[2])
  v.push(sx,sy,0, nsSlope[0],nsSlope[1],nsSlope[2])
  v.push(0,sy,sz, nsSlope[0],nsSlope[1],nsSlope[2])
  v.push(0,0,sz, nsSlope[0],nsSlope[1],nsSlope[2])
  ix.push(si2, si2+1, si2+2, si2, si2+2, si2+3)

  return { v, ix }
}

export function makeTorus(r1: number, r2: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const ringSegs = fn
  // Guard r1<=0 (division by zero → Infinity → infinite loop) and cap the
  // ratio-derived count so a tiny ring radius can't allocate unbounded verts.
  const tubeSegs = Math.max(8, Math.min(MAX_FN, r1 > 0 ? Math.floor(fn * r2 / r1) : 8))
  for (let i = 0; i <= ringSegs; i++) {
    const u = (2 * Math.PI * i) / ringSegs
    const cu = Math.cos(u), su = Math.sin(u)
    for (let j = 0; j <= tubeSegs; j++) {
      const vv = (2 * Math.PI * j) / tubeSegs
      const cv = Math.cos(vv), sv = Math.sin(vv)
      const x = (r1 + r2 * cv) * cu
      const y = r2 * sv
      const z = (r1 + r2 * cv) * su
      // Normal: direction from ring center to surface point
      const nx = cv * cu
      const ny = sv
      const nz = cv * su
      v.push(x, y, z, nx, ny, nz)
    }
  }
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

export function makeHelix(r: number, pitch: number, turns: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const tubeR = pitch * 0.15 // tube radius = 15% of pitch
  turns = Math.min(Math.max(turns, 0), 200) // clamp to prevent unbounded vertex generation
  const totalHeight = pitch * turns
  const ringSegs = Math.min(20000, Math.max(16, fn * turns))
  const tubeSegs = Math.max(6, Math.floor(fn / 4))
  for (let i = 0; i <= ringSegs; i++) {
    const t = i / ringSegs
    const angle = 2 * Math.PI * turns * t
    const ca = Math.cos(angle), sa = Math.sin(angle)
    // Center of tube at this point on helix
    const cx = r * ca
    const cy = totalHeight * t
    const cz = r * sa
    // Tangent to helix path
    const tx = -r * sa * 2 * Math.PI * turns
    const ty = totalHeight
    const tz = r * ca * 2 * Math.PI * turns
    const tlen = Math.sqrt(tx * tx + ty * ty + tz * tz) || 1
    const ttx = tx / tlen, tty = ty / tlen, ttz = tz / tlen
    // Build local frame (normal, binormal)
    // Pick an arbitrary vector not parallel to tangent
    let upx = 0, upy = 1, upz = 0
    if (Math.abs(tty) > 0.9) { upx = 1; upy = 0; upz = 0 }
    // binormal = tangent x up
    let bx = tty * upz - ttz * upy
    let by = ttz * upx - ttx * upz
    let bz = ttx * upy - tty * upx
    const blen = Math.sqrt(bx * bx + by * by + bz * bz) || 1
    bx /= blen; by /= blen; bz /= blen
    // normal = binormal x tangent
    let nx = by * ttz - bz * tty
    let ny = bz * ttx - bx * ttz
    let nz = bx * tty - by * ttx
    const nlen = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
    nx /= nlen; ny /= nlen; nz /= nlen

    for (let j = 0; j <= tubeSegs; j++) {
      const phi = (2 * Math.PI * j) / tubeSegs
      const cp = Math.cos(phi), sp = Math.sin(phi)
      const px = cx + tubeR * (cp * nx + sp * bx)
      const py = cy + tubeR * (cp * ny + sp * by)
      const pz = cz + tubeR * (cp * nz + sp * bz)
      // Surface normal
      const snx = cp * nx + sp * bx
      const sny = cp * ny + sp * by
      const snz = cp * nz + sp * bz
      v.push(px, py, pz, snx, sny, snz)
    }
  }
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

export function makeBezier(points: number[][], thickness: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const tubeR = thickness / 2
  const tubeSegs = Math.max(6, Math.floor(fn / 4))

  // Build cubic Bezier segments: points 0-3, 3-6, 6-9, ...
  const segments: number[][][] = []
  if (points.length <= 4) {
    segments.push(points)
  } else {
    for (let i = 0; i + 3 < points.length; i += 3) {
      segments.push(points.slice(i, i + 4))
    }
    // If remaining points didn't form a complete segment, last segment already covers them
  }

  // Evaluate composite Bezier curve at uniform t in [0, 1]
  const totalSegs = segments.length
  const ringSegs = fn
  const curvePts: number[][] = []
  for (let i = 0; i <= ringSegs; i++) {
    const tGlobal = i / ringSegs
    const segF = tGlobal * totalSegs
    const segIdx = Math.min(Math.floor(segF), totalSegs - 1)
    const t = segF - segIdx
    const seg = segments[segIdx]
    if (seg.length >= 4) {
      // Cubic Bezier: B(t) = (1-t)^3*P0 + 3(1-t)^2*t*P1 + 3(1-t)*t^2*P2 + t^3*P3
      const u = 1 - t
      const u2 = u * u, u3 = u2 * u
      const t2 = t * t, t3 = t2 * t
      curvePts.push([
        u3 * seg[0][0] + 3 * u2 * t * seg[1][0] + 3 * u * t2 * seg[2][0] + t3 * seg[3][0],
        u3 * seg[0][1] + 3 * u2 * t * seg[1][1] + 3 * u * t2 * seg[2][1] + t3 * seg[3][1],
        u3 * seg[0][2] + 3 * u2 * t * seg[1][2] + 3 * u * t2 * seg[2][2] + t3 * seg[3][2],
      ])
    } else if (seg.length === 3) {
      // Quadratic Bezier
      const u = 1 - t
      curvePts.push([
        u * u * seg[0][0] + 2 * u * t * seg[1][0] + t * t * seg[2][0],
        u * u * seg[0][1] + 2 * u * t * seg[1][1] + t * t * seg[2][1],
        u * u * seg[0][2] + 2 * u * t * seg[1][2] + t * t * seg[2][2],
      ])
    } else {
      // Linear interpolation
      const u = 1 - t
      curvePts.push([
        u * seg[0][0] + t * seg[seg.length - 1][0],
        u * seg[0][1] + t * seg[seg.length - 1][1],
        u * seg[0][2] + t * seg[seg.length - 1][2],
      ])
    }
  }

  // Generate tube using Frenet frame (same approach as makeHelix)
  for (let i = 0; i <= ringSegs; i++) {
    const cur = curvePts[i]
    // Compute tangent via finite differences
    let tx: number, ty: number, tz: number
    if (i < ringSegs) {
      const next = curvePts[i + 1]
      tx = next[0] - cur[0]; ty = next[1] - cur[1]; tz = next[2] - cur[2]
    } else {
      const prev = curvePts[i - 1]
      tx = cur[0] - prev[0]; ty = cur[1] - prev[1]; tz = cur[2] - prev[2]
    }
    const tlen = Math.sqrt(tx * tx + ty * ty + tz * tz) || 1
    const ttx = tx / tlen, tty = ty / tlen, ttz = tz / tlen
    // Pick an arbitrary vector not parallel to tangent
    let upx = 0, upy = 1, upz = 0
    if (Math.abs(tty) > 0.9) { upx = 1; upy = 0; upz = 0 }
    // binormal = tangent x up
    let bx = tty * upz - ttz * upy
    let by = ttz * upx - ttx * upz
    let bz = ttx * upy - tty * upx
    const blen = Math.sqrt(bx * bx + by * by + bz * bz) || 1
    bx /= blen; by /= blen; bz /= blen
    // normal = binormal x tangent
    let nx = by * ttz - bz * tty
    let ny = bz * ttx - bx * ttz
    let nz = bx * tty - by * ttx
    const nlen = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
    nx /= nlen; ny /= nlen; nz /= nlen

    for (let j = 0; j <= tubeSegs; j++) {
      const phi = (2 * Math.PI * j) / tubeSegs
      const cp = Math.cos(phi), sp = Math.sin(phi)
      const px = cur[0] + tubeR * (cp * nx + sp * bx)
      const py = cur[1] + tubeR * (cp * ny + sp * by)
      const pz = cur[2] + tubeR * (cp * nz + sp * bz)
      // Surface normal
      const snx = cp * nx + sp * bx
      const sny = cp * ny + sp * by
      const snz = cp * nz + sp * bz
      v.push(px, py, pz, snx, sny, snz)
    }
  }
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

export function makeSweep(path: number[][], radius: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const tubeSegs = fn
  // Cap total ring*tube segment count at 20000 to prevent DoS
  const maxRings = Math.max(2, Math.floor(20000 / Math.max(1, tubeSegs)))
  if (path.length > maxRings) path = path.slice(0, maxRings)
  const ringSegs = path.length - 1

  // Compute tangents for each path point
  const tangents: number[][] = []
  for (let i = 0; i < path.length; i++) {
    let tx: number, ty: number, tz: number
    if (i < path.length - 1) {
      tx = path[i + 1][0] - path[i][0]
      ty = path[i + 1][1] - path[i][1]
      tz = path[i + 1][2] - path[i][2]
    } else {
      // Last point: reuse previous tangent
      tx = tangents[i - 1][0]; ty = tangents[i - 1][1]; tz = tangents[i - 1][2]
    }
    const tlen = Math.sqrt(tx * tx + ty * ty + tz * tz) || 1
    tangents.push([tx / tlen, ty / tlen, tz / tlen])
  }

  // Parallel transport frame
  const normals: number[][] = []
  const binormals: number[][] = []

  // Initialize first frame
  const t0 = tangents[0]
  let upx = 0, upy = 1, upz = 0
  if (Math.abs(t0[1]) > 0.9) { upx = 1; upy = 0; upz = 0 }
  // binormal = tangent x up
  let bx = t0[1] * upz - t0[2] * upy
  let by = t0[2] * upx - t0[0] * upz
  let bz = t0[0] * upy - t0[1] * upx
  let blen = Math.sqrt(bx * bx + by * by + bz * bz) || 1
  bx /= blen; by /= blen; bz /= blen
  // normal = binormal x tangent
  let nx = by * t0[2] - bz * t0[1]
  let ny = bz * t0[0] - bx * t0[2]
  let nz = bx * t0[1] - by * t0[0]
  let nlen = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
  nx /= nlen; ny /= nlen; nz /= nlen
  normals.push([nx, ny, nz])
  binormals.push([bx, by, bz])

  // Propagate frame along path using parallel transport
  for (let i = 1; i < path.length; i++) {
    const tPrev = tangents[i - 1]
    const tCur = tangents[i]
    // Rotation axis = tPrev x tCur
    const ax = tPrev[1] * tCur[2] - tPrev[2] * tCur[1]
    const ay = tPrev[2] * tCur[0] - tPrev[0] * tCur[2]
    const az = tPrev[0] * tCur[1] - tPrev[1] * tCur[0]
    const alen = Math.sqrt(ax * ax + ay * ay + az * az)
    if (alen < 1e-10) {
      // Tangents are parallel, keep previous frame
      normals.push([normals[i - 1][0], normals[i - 1][1], normals[i - 1][2]])
      binormals.push([binormals[i - 1][0], binormals[i - 1][1], binormals[i - 1][2]])
    } else {
      // Rotate previous normal by angle between tangents around rotation axis
      const dot = tPrev[0] * tCur[0] + tPrev[1] * tCur[1] + tPrev[2] * tCur[2]
      const angle = Math.acos(Math.max(-1, Math.min(1, dot)))
      const ux = ax / alen, uy = ay / alen, uz = az / alen
      const cosA = Math.cos(angle), sinA = Math.sin(angle)
      // Rodrigues' rotation formula on the previous normal
      const pn = normals[i - 1]
      const dotUN = ux * pn[0] + uy * pn[1] + uz * pn[2]
      const crossX = uy * pn[2] - uz * pn[1]
      const crossY = uz * pn[0] - ux * pn[2]
      const crossZ = ux * pn[1] - uy * pn[0]
      let rnx = pn[0] * cosA + crossX * sinA + ux * dotUN * (1 - cosA)
      let rny = pn[1] * cosA + crossY * sinA + uy * dotUN * (1 - cosA)
      let rnz = pn[2] * cosA + crossZ * sinA + uz * dotUN * (1 - cosA)
      const rnlen = Math.sqrt(rnx * rnx + rny * rny + rnz * rnz) || 1
      rnx /= rnlen; rny /= rnlen; rnz /= rnlen
      normals.push([rnx, rny, rnz])
      // binormal = tangent x normal
      const rbx = tCur[1] * rnz - tCur[2] * rny
      const rby = tCur[2] * rnx - tCur[0] * rnz
      const rbz = tCur[0] * rny - tCur[1] * rnx
      const rblen = Math.sqrt(rbx * rbx + rby * rby + rbz * rbz) || 1
      binormals.push([rbx / rblen, rby / rblen, rbz / rblen])
    }
  }

  // Generate tube vertices at each path point
  for (let i = 0; i < path.length; i++) {
    const cur = path[i]
    const n = normals[i], b = binormals[i]
    for (let j = 0; j <= tubeSegs; j++) {
      const phi = (2 * Math.PI * j) / tubeSegs
      const cp = Math.cos(phi), sp = Math.sin(phi)
      const px = cur[0] + radius * (cp * n[0] + sp * b[0])
      const py = cur[1] + radius * (cp * n[1] + sp * b[1])
      const pz = cur[2] + radius * (cp * n[2] + sp * b[2])
      // Surface normal
      const snx = cp * n[0] + sp * b[0]
      const sny = cp * n[1] + sp * b[1]
      const snz = cp * n[2] + sp * b[2]
      v.push(px, py, pz, snx, sny, snz)
    }
  }
  // Connect rings with triangles
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

export function makeStar(points: number, r1: number, r2: number, h: number, _fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const n2 = points * 2 // number of vertices around the star profile

  // Build 2D star profile
  const profile: number[][] = []
  for (let i = 0; i < n2; i++) {
    const angle = (i * Math.PI) / points
    const r = (i % 2 === 0) ? r1 : r2
    profile.push([r * Math.cos(angle), r * Math.sin(angle)])
  }

  // Bottom cap (z = 0), normal facing -Z
  const baseBot = v.length / 6
  for (let i = 0; i < n2; i++) {
    v.push(profile[i][0], profile[i][1], 0, 0, 0, -1)
  }
  const botTris = earClip(profile)
  // Reverse winding for bottom cap so normal faces -Z
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap (z = h), normal facing +Z
  const baseTop = v.length / 6
  for (let i = 0; i < n2; i++) {
    v.push(profile[i][0], profile[i][1], h, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < n2; i++) {
    const i2 = (i + 1) % n2
    const x0 = profile[i][0], y0 = profile[i][1]
    const x1 = profile[i2][0], y1 = profile[i2][1]
    // Edge direction
    const ex = x1 - x0, ey = y1 - y0
    // Outward normal (perpendicular to edge, in XY plane)
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nx = ey / len, ny = -ex / len

    const base = v.length / 6
    v.push(x0, y0, 0, nx, ny, 0) // bottom-left
    v.push(x1, y1, 0, nx, ny, 0) // bottom-right
    v.push(x1, y1, h, nx, ny, 0) // top-right
    v.push(x0, y0, h, nx, ny, 0) // top-left
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

export function makePrism(sides: number, r: number, h: number, center: boolean): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const z0 = center ? -h / 2 : 0, z1 = center ? h / 2 : h

  // Build polygon profile
  const profile: number[][] = []
  for (let i = 0; i < sides; i++) {
    const angle = (2 * Math.PI * i) / sides
    profile.push([r * Math.cos(angle), r * Math.sin(angle)])
  }

  // Bottom cap (z0, normal -Z)
  const baseBot = v.length / 6
  for (let i = 0; i < sides; i++) {
    v.push(profile[i][0], profile[i][1], z0, 0, 0, -1)
  }
  const botTris = earClip(profile)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap (z1, normal +Z)
  const baseTop = v.length / 6
  for (let i = 0; i < sides; i++) {
    v.push(profile[i][0], profile[i][1], z1, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < sides; i++) {
    const i2 = (i + 1) % sides
    const x0 = profile[i][0], y0 = profile[i][1]
    const x1 = profile[i2][0], y1 = profile[i2][1]
    const ex = x1 - x0, ey = y1 - y0
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nx = ey / len, ny = -ex / len

    const base = v.length / 6
    v.push(x0, y0, z0, nx, ny, 0)
    v.push(x1, y1, z0, nx, ny, 0)
    v.push(x1, y1, z1, nx, ny, 0)
    v.push(x0, y0, z1, nx, ny, 0)
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

export function makeCapsule(r: number, h: number, center: boolean, fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const totalH = h + 2 * r
  const z0 = center ? -totalH / 2 : 0
  const cylBot = z0 + r
  const cylTop = cylBot + h

  // Cylinder body
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r * ca, r * sa, cylBot, ca, sa, 0)
    v.push(r * ca, r * sa, cylTop, ca, sa, 0)
  }
  for (let i = 0; i < fn; i++) {
    const a2 = i * 2; ix.push(a2, a2 + 1, a2 + 2, a2 + 2, a2 + 1, a2 + 3)
  }

  // Bottom hemisphere (centered at cylBot)
  const halfSeg = Math.max(4, Math.floor(fn / 2))
  const botBase = v.length / 6
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = Math.PI / 2 + (Math.PI / 2) * ri / halfSeg  // PI/2 to PI
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = sp * Math.sin(th), nz = cp
      v.push(r * nx, r * ny, cylBot + r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++)
    for (let si = 0; si < fn; si++) {
      const a = botBase + ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }

  // Top hemisphere (centered at cylTop)
  const topBase = v.length / 6
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = (Math.PI / 2) * ri / halfSeg  // 0 to PI/2
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = sp * Math.sin(th), nz = cp
      v.push(r * nx, r * ny, cylTop + r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++)
    for (let si = 0; si < fn; si++) {
      const a = topBase + ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }

  return { v, ix }
}

export function makeGear(teeth: number, mod: number, thickness: number, fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const tipR = mod * teeth / 2
  const valleyR = tipR - mod
  const n2 = teeth * 2
  const segsPerSide = Math.max(1, fn)

  // Build 2D gear profile: alternating tip/valley arcs
  const profile: number[][] = []
  for (let i = 0; i < n2; i++) {
    const frac = i / n2
    const nextFrac = (i + 1) / n2
    const isTip = i % 2 === 0
    const r1g = isTip ? tipR : valleyR
    const r2g = isTip ? valleyR : tipR
    for (let s = 0; s < segsPerSide; s++) {
      const t = s / segsPerSide
      const angle = 2 * Math.PI * (frac + t * (nextFrac - frac))
      const rr = r1g + (r2g - r1g) * t
      profile.push([rr * Math.cos(angle), rr * Math.sin(angle)])
    }
  }

  const np = profile.length

  // Bottom cap (z=0, normal -Z)
  const baseBot = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(profile[i][0], profile[i][1], 0, 0, 0, -1)
  }
  const botTris = earClip(profile)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap (z=thickness, normal +Z)
  const baseTop = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(profile[i][0], profile[i][1], thickness, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < np; i++) {
    const i2 = (i + 1) % np
    const x0 = profile[i][0], y0 = profile[i][1]
    const x1 = profile[i2][0], y1 = profile[i2][1]
    const ex = x1 - x0, ey = y1 - y0
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nx = ey / len, ny = -ex / len

    const base = v.length / 6
    v.push(x0, y0, 0, nx, ny, 0)
    v.push(x1, y1, 0, nx, ny, 0)
    v.push(x1, y1, thickness, nx, ny, 0)
    v.push(x0, y0, thickness, nx, ny, 0)
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

export function makeThread(d: number, pitch: number, length: number, fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const baseR = d / 2
  const threadDepth = pitch * 0.3
  // Clamp length/pitch ratio and cap total slices to prevent unbounded vertex generation
  const ratio = pitch > 0 ? Math.min(Math.ceil(length / pitch), 20000) : 1
  const zSlices = Math.min(20000, ratio * Math.max(1, Math.floor(fn / 4)))
  const segs = fn // segments around circumference

  // Generate vertices
  for (let zi = 0; zi <= zSlices; zi++) {
    const z = (zi / zSlices) * length
    for (let ti = 0; ti <= segs; ti++) {
      const theta = (2 * Math.PI * ti) / segs
      const sinVal = Math.sin(2 * Math.PI * (z / pitch - theta / (2 * Math.PI)))
      const modulation = Math.max(0, sinVal)
      const r = baseR - threadDepth * modulation

      const x = r * Math.cos(theta)
      const y = r * Math.sin(theta)

      // Approximate normal by computing partial derivatives
      // Radial direction gives the main normal component
      const drdTheta = -threadDepth * Math.max(0, Math.cos(2 * Math.PI * (z / pitch - theta / (2 * Math.PI)))) * (sinVal > 0 ? 1 : 0) / (2 * Math.PI) * (2 * Math.PI) / (2 * Math.PI)
      const nx0 = Math.cos(theta)
      const ny0 = Math.sin(theta)
      // For simplicity, use the radial normal adjusted slightly
      // The tangent along theta: (-r*sin(theta) + drdTheta*cos(theta), r*cos(theta) + drdTheta*sin(theta), 0)
      // The tangent along z: (drdz*cos(theta), drdz*sin(theta), 1)
      // Normal = cross(tangent_theta, tangent_z)
      const drdz_raw = -threadDepth * Math.cos(2 * Math.PI * (z / pitch - theta / (2 * Math.PI))) * (2 * Math.PI / pitch) * (sinVal > 0 ? 1 : 0)
      const drdz = sinVal > 0 ? drdz_raw : 0

      // tangent along theta
      const ttx = -r * Math.sin(theta) + drdTheta * Math.cos(theta)
      const tty = r * Math.cos(theta) + drdTheta * Math.sin(theta)
      const ttz = 0
      // tangent along z
      const tzx = drdz * Math.cos(theta)
      const tzy = drdz * Math.sin(theta)
      const tzz = 1

      // cross product: tangent_theta x tangent_z
      let cnx = tty * tzz - ttz * tzy
      let cny = ttz * tzx - ttx * tzz
      let cnz = ttx * tzy - tty * tzx

      const clen = Math.sqrt(cnx * cnx + cny * cny + cnz * cnz) || 1
      cnx /= clen; cny /= clen; cnz /= clen

      // Ensure normal points outward (dot with radial direction should be positive)
      if (cnx * nx0 + cny * ny0 < 0) {
        cnx = -cnx; cny = -cny; cnz = -cnz
      }

      v.push(x, y, z, cnx, cny, cnz)
    }
  }

  // Generate indices
  for (let zi = 0; zi < zSlices; zi++) {
    for (let ti = 0; ti < segs; ti++) {
      const a = zi * (segs + 1) + ti
      const b = a + segs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Bottom cap (z = 0)
  const botCenter = v.length / 6
  v.push(0, 0, 0, 0, 0, -1)
  const botRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const sinVal = Math.sin(2 * Math.PI * (0 / pitch - theta / (2 * Math.PI)))
    const modulation = Math.max(0, sinVal)
    const r = baseR - threadDepth * modulation
    v.push(r * Math.cos(theta), r * Math.sin(theta), 0, 0, 0, -1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(botCenter, botRing + next, botRing + ti)
  }

  // Top cap (z = length)
  const topCenter = v.length / 6
  v.push(0, 0, length, 0, 0, 1)
  const topRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const sinVal = Math.sin(2 * Math.PI * (length / pitch - theta / (2 * Math.PI)))
    const modulation = Math.max(0, sinVal)
    const r = baseR - threadDepth * modulation
    v.push(r * Math.cos(theta), r * Math.sin(theta), length, 0, 0, 1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(topCenter, topRing + ti, topRing + next)
  }

  return { v, ix }
}

export function pointInTriangle(px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number): boolean {
  // Strict containment: a point lying on an edge of the triangle does NOT count
  // as inside (so it never blocks an otherwise-valid ear). A point counts as
  // inside only if it is on the same side of every edge by more than epsilon.
  const eps = 1e-9
  const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by)
  const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy)
  const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay)
  const hasNeg = (d1 < -eps) || (d2 < -eps) || (d3 < -eps)
  const hasPos = (d1 > eps) || (d2 > eps) || (d3 > eps)
  return !(hasNeg && hasPos)
}

export function earClip(pts: number[][]): number[] {
  const n = pts.length
  if (n < 3) return []
  if (n === 3) return [0, 1, 2]

  // Determine winding order (positive = CCW)
  let area = 0
  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n
    area += (pts[i][0] ?? 0) * (pts[j][1] ?? 0)
    area -= (pts[j][0] ?? 0) * (pts[i][1] ?? 0)
  }
  const ccw = area > 0

  const remaining = Array.from({ length: n }, (_, i) => i)
  const tris: number[] = []

  // Epsilon for treating a candidate ear's area as ~0 (collinear / degenerate).
  const areaEps = 1e-9

  while (remaining.length > 2) {
    let found = false
    for (let i = 0; i < remaining.length; i++) {
      const prev = remaining[(i - 1 + remaining.length) % remaining.length]
      const cur = remaining[i]
      const next = remaining[(i + 1) % remaining.length]

      const ax = pts[prev][0] ?? 0, ay = pts[prev][1] ?? 0
      const bx = pts[cur][0] ?? 0, by = pts[cur][1] ?? 0
      const cx = pts[next][0] ?? 0, cy = pts[next][1] ?? 0

      // Cross product to check if ear is convex
      const cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)

      // Skip near-degenerate (collinear / zero-area) candidate ears so the
      // algorithm doesn't stall on them.
      if (Math.abs(cross) < areaEps) continue

      const isConvex = ccw ? cross > 0 : cross < 0
      if (!isConvex) continue

      // Check no other point is strictly inside this triangle
      let hasInside = false
      for (const idx of remaining) {
        if (idx === prev || idx === cur || idx === next) continue
        const ppx = pts[idx][0] ?? 0, ppy = pts[idx][1] ?? 0
        if (pointInTriangle(ppx, ppy, ax, ay, bx, by, cx, cy)) {
          hasInside = true
          break
        }
      }
      if (hasInside) continue

      tris.push(prev, cur, next)
      remaining.splice(i, 1)
      found = true
      break
    }

    // No valid ear found in a full pass but vertices remain (self-intersecting,
    // collinear, or otherwise degenerate polygon). Fall back to a fan
    // triangulation of the remaining vertices so the cap is fully covered
    // (no holes) rather than returning a partial result.
    if (!found) {
      const anchor = remaining[0]
      for (let i = 1; i < remaining.length - 1; i++) {
        tris.push(anchor, remaining[i], remaining[i + 1])
      }
      break
    }
  }

  return tris
}

export function makePolygon(points: number[][]) {
  if (points.length < 3) return { v: [] as number[], ix: [] as number[] }
  const h = 0.01
  const v: number[] = []
  const ix: number[] = []

  // Triangulate using ear clipping
  const indices = earClip(points)

  // Top face (z = h)
  for (const p of points) {
    v.push(p[0] ?? 0, p[1] ?? 0, h, 0, 0, 1)
  }
  for (let i = 0; i < indices.length; i += 3) {
    ix.push(indices[i], indices[i+1], indices[i+2])
  }

  // Bottom face (z = 0)
  const bOff = points.length
  for (const p of points) {
    v.push(p[0] ?? 0, p[1] ?? 0, 0, 0, 0, -1)
  }
  for (let i = 0; i < indices.length; i += 3) {
    ix.push(bOff + indices[i], bOff + indices[i+2], bOff + indices[i+1])
  }

  return { v, ix }
}

export function makeHoneycomb(rows: number, cols: number, r: number, h: number, wall: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const sqrt3 = Math.sqrt(3)
  // Spacing between hex centers
  const xSpacing = r * 1.5 + wall
  const ySpacing = r * sqrt3 + wall
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const cx = col * xSpacing
      const cy = row * ySpacing + (col % 2) * (ySpacing / 2)
      // Create hexagonal prism at (cx, cy)
      const hexVerts: number[][] = []
      for (let i = 0; i < 6; i++) {
        const angle = (Math.PI / 3) * i + Math.PI / 6
        hexVerts.push([cx + r * Math.cos(angle), cy + r * Math.sin(angle)])
      }
      // Bottom cap
      const baseBot = v.length / 6
      for (let i = 0; i < 6; i++) {
        v.push(hexVerts[i][0], hexVerts[i][1], 0, 0, 0, -1)
      }
      // Fan triangulation for bottom (reversed winding)
      for (let i = 1; i < 5; i++) {
        ix.push(baseBot, baseBot + i + 1, baseBot + i)
      }
      // Top cap
      const baseTop = v.length / 6
      for (let i = 0; i < 6; i++) {
        v.push(hexVerts[i][0], hexVerts[i][1], h, 0, 0, 1)
      }
      for (let i = 1; i < 5; i++) {
        ix.push(baseTop, baseTop + i, baseTop + i + 1)
      }
      // Side walls
      for (let i = 0; i < 6; i++) {
        const i2 = (i + 1) % 6
        const x0 = hexVerts[i][0], y0 = hexVerts[i][1]
        const x1 = hexVerts[i2][0], y1 = hexVerts[i2][1]
        const ex = x1 - x0, ey = y1 - y0
        const len = Math.sqrt(ex * ex + ey * ey) || 1
        const nx = ey / len, ny = -ex / len
        const base = v.length / 6
        v.push(x0, y0, 0, nx, ny, 0)
        v.push(x1, y1, 0, nx, ny, 0)
        v.push(x1, y1, h, nx, ny, 0)
        v.push(x0, y0, h, nx, ny, 0)
        ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
      }
    }
  }
  return { v, ix }
}

export function makeKnurl(d: number, h: number, pitch: number, depth: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const baseR = d / 2
  const nTeeth = Math.max(1, Math.round(Math.PI * d / pitch))
  const nVertical = Math.max(1, Math.round(h / pitch))
  const zSlices = Math.max(8, fn)
  const segs = fn

  for (let zi = 0; zi <= zSlices; zi++) {
    const z = (zi / zSlices) * h
    for (let ti = 0; ti <= segs; ti++) {
      const theta = (2 * Math.PI * ti) / segs
      // Diamond knurl: two crossed sine waves
      const mod1 = Math.sin(nTeeth * theta)
      const mod2 = Math.sin(nVertical * 2 * Math.PI * z / h)
      const r = baseR + depth * mod1 * mod2

      const x = r * Math.cos(theta)
      const y = r * Math.sin(theta)

      // Approximate normal (radial)
      const nx0 = Math.cos(theta)
      const ny0 = Math.sin(theta)
      v.push(x, y, z, nx0, ny0, 0)
    }
  }

  for (let zi = 0; zi < zSlices; zi++) {
    for (let ti = 0; ti < segs; ti++) {
      const a = zi * (segs + 1) + ti
      const b = a + segs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Bottom cap
  const botCenter = v.length / 6
  v.push(0, 0, 0, 0, 0, -1)
  const botRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const mod1 = Math.sin(nTeeth * theta)
    const mod2 = Math.sin(0) // z=0
    const r = baseR + depth * mod1 * mod2
    v.push(r * Math.cos(theta), r * Math.sin(theta), 0, 0, 0, -1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(botCenter, botRing + next, botRing + ti)
  }

  // Top cap
  const topCenter = v.length / 6
  v.push(0, 0, h, 0, 0, 1)
  const topRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const mod1 = Math.sin(nTeeth * theta)
    const mod2 = Math.sin(nVertical * 2 * Math.PI)
    const r = baseR + depth * mod1 * mod2
    v.push(r * Math.cos(theta), r * Math.sin(theta), h, 0, 0, 1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(topCenter, topRing + ti, topRing + next)
  }

  return { v, ix }
}

export function makeChamferCube(sx: number, sy: number, sz: number, chamfer: number, center: boolean): { v: number[]; ix: number[] } {
  const c = Math.min(chamfer, sx / 2, sy / 2, sz / 2)
  const x0 = center ? -sx / 2 : 0
  const y0 = center ? -sy / 2 : 0
  const z0 = center ? -sz / 2 : 0
  const x1 = x0 + sx
  const y1 = y0 + sy
  const z1 = z0 + sz

  // 24 vertices: for each of 8 corners, 3 vertices offset inward along each edge
  const points: number[][] = []
  const corners: [number, number, number][] = [
    [x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0],
    [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1],
  ]
  // For each corner, generate 3 chamfer vertices (offset along each adjacent edge)
  const dirs: [number, number, number][][] = [
    [[1,0,0],[0,1,0],[0,0,1]],   // corner 0 (x0,y0,z0)
    [[-1,0,0],[0,1,0],[0,0,1]],  // corner 1 (x1,y0,z0)
    [[-1,0,0],[0,-1,0],[0,0,1]], // corner 2 (x1,y1,z0)
    [[1,0,0],[0,-1,0],[0,0,1]],  // corner 3 (x0,y1,z0)
    [[1,0,0],[0,1,0],[0,0,-1]],  // corner 4 (x0,y0,z1)
    [[-1,0,0],[0,1,0],[0,0,-1]], // corner 5 (x1,y0,z1)
    [[-1,0,0],[0,-1,0],[0,0,-1]],// corner 6 (x1,y1,z1)
    [[1,0,0],[0,-1,0],[0,0,-1]], // corner 7 (x0,y1,z1)
  ]

  for (let ci = 0; ci < 8; ci++) {
    const [cx, cy, cz] = corners[ci]
    for (const [dx, dy, dz] of dirs[ci]) {
      points.push([cx + c * dx, cy + c * dy, cz + c * dz])
    }
  }

  // Build faces. Each original face is a rectangle with corners cut.
  // The 24 vertices are indexed as: corner i => vertices 3*i, 3*i+1, 3*i+2
  // where vertex 3*i+j is offset along direction j of that corner.
  // Faces:
  // Bottom (z0): corners 0,1,2,3 - use z-edge vertices (index +2 for each corner)
  //   but actually we need the vertices that lie on the bottom face.
  //   For corner 0: the x-offset (idx 0) and y-offset (idx 1) lie on z=z0
  //   For corner 1: the x-offset (idx 3) and y-offset (idx 4) lie on z=z0
  //   etc.
  const faceDefs: { verts: number[]; nx: number; ny: number; nz: number }[] = [
    // Bottom face (z=z0): vertices that have z=z0, i.e. x and y offsets of bottom corners
    { verts: [0,1, 4,3, 7,6, 10,11], nx: 0, ny: 0, nz: -1 },
    // Top face (z=z1): vertices that have z=z1
    { verts: [12,13, 16,15, 19,18, 22,23], nx: 0, ny: 0, nz: 1 },
    // Front face (y=y0): corners 0,1,5,4, y-offset vertices
    { verts: [1,2, 5,3, 14,15, 13,12], nx: 0, ny: -1, nz: 0 },
    // Wait, let me re-index properly...
  ]
  void faceDefs

  // Simpler approach: use convex hull on the 24 points
  const hullPts: [number, number, number][] = points.map(p => [p[0], p[1], p[2]])
  return convexHull3D(hullPts)
}

export function makeLoft(profiles: number[][][], heights: number[], fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  if (profiles.length < 2 || heights.length < 2) return { v, ix }

  // Ensure all profiles have the same number of vertices by resampling
  const maxVerts = Math.max(...profiles.map(p => p.length))
  const targetVerts = Math.max(maxVerts, fn)

  // Resample a profile to have exactly n points
  function resampleProfile(profile: number[][], n: number): number[][] {
    if (profile.length === n) return profile
    const result: number[][] = []
    const len = profile.length
    for (let i = 0; i < n; i++) {
      const t = (i / n) * len
      const idx = Math.floor(t)
      const frac = t - idx
      const p0 = profile[idx % len]
      const p1 = profile[(idx + 1) % len]
      result.push([
        p0[0] + frac * (p1[0] - p0[0]),
        p0[1] + frac * (p1[1] - p0[1]),
      ])
    }
    return result
  }

  const resampledProfiles = profiles.map(p => resampleProfile(p, targetVerts))

  // Generate vertices for each profile at its height
  for (let pi = 0; pi < resampledProfiles.length; pi++) {
    const profile = resampledProfiles[pi]
    const z = heights[pi] ?? (pi * 10)
    for (let vi2 = 0; vi2 < targetVerts; vi2++) {
      const x = profile[vi2][0]
      const y = profile[vi2][1]
      // Approximate normal: outward from centroid
      let cx = 0, cy = 0
      for (const pt of profile) { cx += pt[0]; cy += pt[1] }
      cx /= profile.length; cy /= profile.length
      const dx = x - cx, dy = y - cy
      const nl = Math.sqrt(dx * dx + dy * dy) || 1
      v.push(x, y, z, dx / nl, dy / nl, 0)
    }
  }

  // Connect adjacent profiles with quads
  for (let pi = 0; pi < resampledProfiles.length - 1; pi++) {
    for (let vi2 = 0; vi2 < targetVerts; vi2++) {
      const nextVi = (vi2 + 1) % targetVerts
      const a = pi * targetVerts + vi2
      const b = pi * targetVerts + nextVi
      const c = (pi + 1) * targetVerts + vi2
      const d = (pi + 1) * targetVerts + nextVi
      ix.push(a, b, d, a, d, c)
    }
  }

  // Bottom cap
  const botOff = v.length / 6
  const botProfile = resampledProfiles[0]
  const botZ = heights[0] ?? 0
  for (let i = 0; i < targetVerts; i++) {
    v.push(botProfile[i][0], botProfile[i][1], botZ, 0, 0, -1)
  }
  const botTris = earClip(botProfile)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(botOff + botTris[i], botOff + botTris[i + 2], botOff + botTris[i + 1])
  }

  // Top cap
  const topOff = v.length / 6
  const topProfile = resampledProfiles[resampledProfiles.length - 1]
  const topZ = heights[heights.length - 1] ?? ((resampledProfiles.length - 1) * 10)
  for (let i = 0; i < targetVerts; i++) {
    v.push(topProfile[i][0], topProfile[i][1], topZ, 0, 0, 1)
  }
  const topTris = earClip(topProfile)
  for (let i = 0; i < topTris.length; i += 3) {
    ix.push(topOff + topTris[i], topOff + topTris[i + 1], topOff + topTris[i + 2])
  }

  return { v, ix }
}

export function makeSurface(data: number[][]): { v: number[]; ix: number[] } {
  const rows = data.length
  if (rows < 2) return { v: [], ix: [] }
  const cols = data[0].length
  if (cols < 2) return { v: [], ix: [] }

  const v: number[] = []
  const ix: number[] = []

  // Generate vertices with normals
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const z = typeof data[y][x] === 'number' ? data[y][x] : 0
      // Compute normal via finite differences
      const zL = x > 0 ? (typeof data[y][x - 1] === 'number' ? data[y][x - 1] : z) : z
      const zR = x < cols - 1 ? (typeof data[y][x + 1] === 'number' ? data[y][x + 1] : z) : z
      const zD = y > 0 ? (typeof data[y - 1][x] === 'number' ? data[y - 1][x] : z) : z
      const zU = y < rows - 1 ? (typeof data[y + 1][x] === 'number' ? data[y + 1][x] : z) : z
      const dx = (zR - zL) / (x > 0 && x < cols - 1 ? 2 : 1)
      const dy = (zU - zD) / (y > 0 && y < rows - 1 ? 2 : 1)
      // Normal = cross product of tangent vectors
      // tangent_x = (1, 0, dx), tangent_y = (0, 1, dy)
      // normal = (-dx, -dy, 1) normalized
      const len = Math.sqrt(dx * dx + dy * dy + 1)
      const nx = -dx / len, ny = -dy / len, nz = 1 / len
      v.push(x, y, z, nx, ny, nz)
    }
  }

  // Generate triangles - two per cell
  for (let y = 0; y < rows - 1; y++) {
    for (let x = 0; x < cols - 1; x++) {
      const i00 = y * cols + x
      const i10 = y * cols + (x + 1)
      const i01 = (y + 1) * cols + x
      const i11 = (y + 1) * cols + (x + 1)
      ix.push(i00, i10, i11)
      ix.push(i00, i11, i01)
    }
  }

  return { v, ix }
}

/* ── Expression evaluator ─────────────────────────── */


export function extractFlatProfile(mesh: MeshData): { pts: [number, number][]; flat: boolean } {
  const verts = mesh.vertices
  const nv = verts.length / 6
  let minZ = Infinity, maxZ = -Infinity
  const pts: [number, number][] = []

  for (let i = 0; i < nv; i++) {
    const z = verts[i * 6 + 2]
    if (z < minZ) minZ = z
    if (z > maxZ) maxZ = z
  }

  const flat = (maxZ - minZ) < 0.5

  if (!flat) return { pts: [], flat: false }

  // Collect unique XY points from the mesh edges
  const idxs = mesh.indices
  const edgeSet = new Set<string>()
  const edgeVerts: [number, number][] = []

  for (let i = 0; i < nv; i++) {
    const x = verts[i * 6]
    const y = verts[i * 6 + 1]
    const key = `${x.toFixed(4)},${y.toFixed(4)}`
    if (!edgeSet.has(key)) {
      edgeSet.add(key)
      edgeVerts.push([x, y])
    }
  }

  // Build boundary edges: edges that appear in only one triangle
  const edgeCount = new Map<string, number>()
  const triCount = idxs.length / 3
  for (let t = 0; t < triCount; t++) {
    const i0 = idxs[t * 3], i1 = idxs[t * 3 + 1], i2 = idxs[t * 3 + 2]
    const edges = [[i0, i1], [i1, i2], [i2, i0]]
    for (const [a, b] of edges) {
      const key = a < b ? `${a},${b}` : `${b},${a}`
      edgeCount.set(key, (edgeCount.get(key) || 0) + 1)
    }
  }

  // Collect boundary vertex indices
  const boundaryVertSet = new Set<number>()
  for (const [key, count] of edgeCount) {
    if (count === 1) {
      const [a, b] = key.split(',').map(Number)
      boundaryVertSet.add(a)
      boundaryVertSet.add(b)
    }
  }

  // If we have boundary vertices, use them; otherwise use all unique verts
  const boundaryPts: [number, number][] = []
  if (boundaryVertSet.size >= 3) {
    // Build adjacency for boundary edges to order them
    const adj = new Map<number, number[]>()
    for (const [key, count] of edgeCount) {
      if (count === 1) {
        const [a, b] = key.split(',').map(Number)
        if (!adj.has(a)) adj.set(a, [])
        if (!adj.has(b)) adj.set(b, [])
        adj.get(a)!.push(b)
        adj.get(b)!.push(a)
      }
    }

    // Walk the boundary
    const visited = new Set<number>()
    let current = boundaryVertSet.values().next().value
    if (current !== undefined) {
      while (current !== undefined && !visited.has(current)) {
        visited.add(current)
        boundaryPts.push([verts[current * 6], verts[current * 6 + 1]])
        const nbrs: number[] = adj.get(current) || []
        let next: number | undefined = undefined
        for (const nb of nbrs) {
          if (!visited.has(nb)) { next = nb; break }
        }
        current = next
      }
    }
  }

  if (boundaryPts.length >= 3) {
    return { pts: boundaryPts, flat: true }
  }

  // Fallback: sort by angle around centroid
  if (edgeVerts.length < 3) return { pts: edgeVerts, flat: true }
  let cx = 0, cy = 0
  for (const [x, y] of edgeVerts) { cx += x; cy += y }
  cx /= edgeVerts.length; cy /= edgeVerts.length
  edgeVerts.sort((a, b) => Math.atan2(a[1] - cy, a[0] - cx) - Math.atan2(b[1] - cy, b[0] - cx))

  return { pts: edgeVerts, flat: true }
}

/** Generate extruded mesh from a 2D profile */
export function extrudeMesh(profile: [number, number][], height: number, twist: number, slices: number): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  const n = profile.length
  if (n < 3) return { v, ix }

  // Cap slices: an unbounded twist (e.g. twist=2e6) would otherwise derive
  // hundreds of thousands of slices and stall the main thread.
  const actualSlices = Math.min(512, Math.max(1, twist !== 0 ? Math.max(slices, Math.ceil(Math.abs(twist) / 10)) : slices))

  // Generate vertices for each slice
  for (let s = 0; s <= actualSlices; s++) {
    const t = s / actualSlices
    const z = height * t
    const angle = (twist * t) * Math.PI / 180

    const ca = Math.cos(angle), sa = Math.sin(angle)

    for (let i = 0; i < n; i++) {
      const x = profile[i][0], y = profile[i][1]
      const rx = x * ca - y * sa
      const ry = x * sa + y * ca
      // Normal: pointing outward (approximate)
      const next = profile[(i + 1) % n]
      const dx = next[0] - profile[i][0]
      const dy = next[1] - profile[i][1]
      const nl = Math.sqrt(dx * dx + dy * dy) || 1
      let nx = dy / nl, ny = -dx / nl
      // Rotate normal too
      const rnx = nx * ca - ny * sa
      const rny = nx * sa + ny * ca
      v.push(rx, ry, z, rnx, rny, 0)
    }
  }

  // Side faces
  for (let s = 0; s < actualSlices; s++) {
    for (let i = 0; i < n; i++) {
      const ni = (i + 1) % n
      const a = s * n + i
      const b = s * n + ni
      const c = (s + 1) * n + i
      const d = (s + 1) * n + ni
      ix.push(a, b, d, a, d, c)
    }
  }

  // Bottom cap (z = 0)
  const bottomOff = v.length / 6
  for (let i = 0; i < n; i++) {
    v.push(profile[i][0], profile[i][1], 0, 0, 0, -1)
  }
  const bottomTris = earClip(profile.map(p => [p[0], p[1]]))
  for (let i = 0; i < bottomTris.length; i += 3) {
    ix.push(bottomOff + bottomTris[i], bottomOff + bottomTris[i + 2], bottomOff + bottomTris[i + 1])
  }

  // Top cap (z = height)
  const topOff = v.length / 6
  const topAngle = twist * Math.PI / 180
  const tca = Math.cos(topAngle), tsa = Math.sin(topAngle)
  const topProfile: [number, number][] = []
  for (let i = 0; i < n; i++) {
    const x = profile[i][0], y = profile[i][1]
    const rx = x * tca - y * tsa
    const ry = x * tsa + y * tca
    topProfile.push([rx, ry])
    v.push(rx, ry, height, 0, 0, 1)
  }
  const topTris = earClip(topProfile.map(p => [p[0], p[1]]))
  for (let i = 0; i < topTris.length; i += 3) {
    ix.push(topOff + topTris[i], topOff + topTris[i + 1], topOff + topTris[i + 2])
  }

  return { v, ix }
}

/** Generate a rotation-extruded mesh from a 2D XZ profile */
export function rotateExtrudeMesh(profile: [number, number][], fn: number, angle: number): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  const n = profile.length
  if (n < 2) return { v, ix }

  const steps = Math.max(3, fn)
  const fullAngle = angle * Math.PI / 180

  // Generate vertices by rotating the profile around Z axis
  for (let s = 0; s <= steps; s++) {
    const t = s / steps
    const a = fullAngle * t
    const ca = Math.cos(a), sa = Math.sin(a)

    for (let i = 0; i < n; i++) {
      const r = profile[i][0]  // X coordinate = radius
      const z = profile[i][1]  // Y coordinate = Z height

      const x = r * ca
      const y = r * sa

      // Normal: approximate
      const prev = i > 0 ? i - 1 : i
      const next = i < n - 1 ? i + 1 : i
      const dr = profile[next][0] - profile[prev][0]
      const dz = profile[next][1] - profile[prev][1]
      const nl = Math.sqrt(dr * dr + dz * dz) || 1
      const nr = dz / nl   // outward in R direction
      const nz = -dr / nl  // Z component

      v.push(x, y, z, nr * ca, nr * sa, nz)
    }
  }

  // Generate faces
  for (let s = 0; s < steps; s++) {
    for (let i = 0; i < n - 1; i++) {
      const a0 = s * n + i
      const b0 = s * n + i + 1
      const a1 = (s + 1) * n + i
      const b1 = (s + 1) * n + i + 1
      ix.push(a0, b0, b1, a0, b1, a1)
    }
  }

  // If full 360, don't cap; if partial angle, add caps
  if (Math.abs(angle - 360) > 0.01) {
    // Start cap (angle = 0)
    const startOff = v.length / 6
    for (let i = 0; i < n; i++) {
      v.push(profile[i][0], 0, profile[i][1], 0, -1, 0)
    }
    const capTris = earClip(profile.map(p => [p[0], p[1]]))
    for (let i = 0; i < capTris.length; i += 3) {
      ix.push(startOff + capTris[i], startOff + capTris[i + 2], startOff + capTris[i + 1])
    }

    // End cap
    const endAngle = fullAngle
    const eca = Math.cos(endAngle), esa = Math.sin(endAngle)
    const endOff = v.length / 6
    for (let i = 0; i < n; i++) {
      const r = profile[i][0]
      const x = r * eca, y = r * esa
      // Normal perpendicular to the end face
      const nx = -esa, ny = eca
      v.push(x, y, profile[i][1], nx, ny, 0)
    }
    for (let i = 0; i < capTris.length; i += 3) {
      ix.push(endOff + capTris[i], endOff + capTris[i + 1], endOff + capTris[i + 2])
    }
  }

  return { v, ix }
}

/* ── 3D Convex Hull ───────────────────────────────── */

interface HullFace {
  a: number
  b: number
  c: number
  nx: number
  ny: number
  nz: number
  d: number // plane distance
}

export function computeHullFaceNormal(pts: Vec3[], a: number, b: number, c: number): { nx: number; ny: number; nz: number; d: number } {
  const ax = pts[b][0] - pts[a][0], ay = pts[b][1] - pts[a][1], az = pts[b][2] - pts[a][2]
  const bx = pts[c][0] - pts[a][0], by = pts[c][1] - pts[a][1], bz = pts[c][2] - pts[a][2]
  let nx = ay * bz - az * by
  let ny = az * bx - ax * bz
  let nz = ax * by - ay * bx
  const len = Math.sqrt(nx * nx + ny * ny + nz * nz)
  if (len > 1e-10) { nx /= len; ny /= len; nz /= len }
  const d = nx * pts[a][0] + ny * pts[a][1] + nz * pts[a][2]
  return { nx, ny, nz, d }
}

export function convexHull3D(points: Vec3[]): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []

  if (points.length < 4) {
    // Not enough for a 3D hull; make a degenerate shape
    if (points.length === 0) return { v, ix }
    // Build a tiny cube around center of given points
    let cx = 0, cy = 0, cz = 0
    for (const p of points) { cx += p[0]; cy += p[1]; cz += p[2] }
    cx /= points.length; cy /= points.length; cz /= points.length
    return makeCube(0.1, 0.1, 0.1, true)
  }

  // Remove duplicate points
  const uniqueMap = new Map<string, number>()
  const pts: Vec3[] = []
  for (const p of points) {
    const key = `${p[0].toFixed(6)},${p[1].toFixed(6)},${p[2].toFixed(6)}`
    if (!uniqueMap.has(key)) {
      uniqueMap.set(key, pts.length)
      pts.push(p)
    }
  }

  if (pts.length < 4) {
    // Degenerate: make bounding box
    let minX = Infinity, minY = Infinity, minZ = Infinity
    let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity
    for (const p of pts) {
      if (p[0] < minX) minX = p[0]; if (p[0] > maxX) maxX = p[0]
      if (p[1] < minY) minY = p[1]; if (p[1] > maxY) maxY = p[1]
      if (p[2] < minZ) minZ = p[2]; if (p[2] > maxZ) maxZ = p[2]
    }
    const sx = Math.max(maxX - minX, 0.1)
    const sy = Math.max(maxY - minY, 0.1)
    const sz = Math.max(maxZ - minZ, 0.1)
    const cube = makeCube(sx, sy, sz, false)
    // Offset vertices
    for (let i = 0; i < cube.v.length; i += 6) {
      cube.v[i] += minX; cube.v[i+1] += minY; cube.v[i+2] += minZ
    }
    return cube
  }

  // Find 4 non-coplanar points for initial tetrahedron
  let i0 = 0, i1 = -1, i2 = -1, i3 = -1

  // Find most distant point from i0
  let maxDist = 0
  for (let i = 1; i < pts.length; i++) {
    const dx = pts[i][0] - pts[0][0], dy = pts[i][1] - pts[0][1], dz = pts[i][2] - pts[0][2]
    const dist = dx*dx + dy*dy + dz*dz
    if (dist > maxDist) { maxDist = dist; i1 = i }
  }
  if (i1 === -1) i1 = 1

  // Find point most distant from line i0-i1
  maxDist = 0
  const lx = pts[i1][0] - pts[i0][0], ly = pts[i1][1] - pts[i0][1], lz = pts[i1][2] - pts[i0][2]
  for (let i = 0; i < pts.length; i++) {
    if (i === i0 || i === i1) continue
    const dx = pts[i][0] - pts[i0][0], dy = pts[i][1] - pts[i0][1], dz = pts[i][2] - pts[i0][2]
    // Cross product magnitude
    const cx2 = dy*lz - dz*ly, cy2 = dz*lx - dx*lz, cz2 = dx*ly - dy*lx
    const dist = cx2*cx2 + cy2*cy2 + cz2*cz2
    if (dist > maxDist) { maxDist = dist; i2 = i }
  }
  if (i2 === -1) i2 = 2 < pts.length ? 2 : i1

  // Find point most distant from plane i0-i1-i2
  const { nx: pnx, ny: pny, nz: pnz, d: pd } = computeHullFaceNormal(pts, i0, i1, i2)
  maxDist = 0
  for (let i = 0; i < pts.length; i++) {
    if (i === i0 || i === i1 || i === i2) continue
    const dist = Math.abs(pnx * pts[i][0] + pny * pts[i][1] + pnz * pts[i][2] - pd)
    if (dist > maxDist) { maxDist = dist; i3 = i }
  }
  if (i3 === -1) {
    // All coplanar - build a flat prism as approximation
    let minXf = Infinity, minYf = Infinity, minZf = Infinity
    let maxXf = -Infinity, maxYf = -Infinity, maxZf = -Infinity
    for (const p of pts) {
      if (p[0] < minXf) minXf = p[0]; if (p[0] > maxXf) maxXf = p[0]
      if (p[1] < minYf) minYf = p[1]; if (p[1] > maxYf) maxYf = p[1]
      if (p[2] < minZf) minZf = p[2]; if (p[2] > maxZf) maxZf = p[2]
    }
    const cube = makeCube(Math.max(maxXf - minXf, 0.1), Math.max(maxYf - minYf, 0.1), Math.max(maxZf - minZf, 0.1), false)
    for (let i = 0; i < cube.v.length; i += 6) {
      cube.v[i] += minXf; cube.v[i+1] += minYf; cube.v[i+2] += minZf
    }
    return cube
  }

  // Build initial tetrahedron with correct winding
  // Ensure i3 is on the positive side of face (i0,i1,i2)
  const side = pnx * pts[i3][0] + pny * pts[i3][1] + pnz * pts[i3][2] - pd
  let faces: HullFace[]
  if (side > 0) {
    // i3 above plane -> flip face orientation
    faces = [
      { ...computeHullFaceNormal(pts, i0, i2, i1), a: i0, b: i2, c: i1 },
      { ...computeHullFaceNormal(pts, i0, i1, i3), a: i0, b: i1, c: i3 },
      { ...computeHullFaceNormal(pts, i1, i2, i3), a: i1, b: i2, c: i3 },
      { ...computeHullFaceNormal(pts, i2, i0, i3), a: i2, b: i0, c: i3 },
    ]
  } else {
    faces = [
      { ...computeHullFaceNormal(pts, i0, i1, i2), a: i0, b: i1, c: i2 },
      { ...computeHullFaceNormal(pts, i0, i3, i1), a: i0, b: i3, c: i1 },
      { ...computeHullFaceNormal(pts, i1, i3, i2), a: i1, b: i3, c: i2 },
      { ...computeHullFaceNormal(pts, i2, i3, i0), a: i2, b: i3, c: i0 },
    ]
  }

  // Verify normals point outward: for each face, center of all other initial points should be inside
  const tetCenter: Vec3 = [
    (pts[i0][0]+pts[i1][0]+pts[i2][0]+pts[i3][0])/4,
    (pts[i0][1]+pts[i1][1]+pts[i2][1]+pts[i3][1])/4,
    (pts[i0][2]+pts[i1][2]+pts[i2][2]+pts[i3][2])/4,
  ]
  for (let fi = 0; fi < faces.length; fi++) {
    const f = faces[fi]
    const dot = f.nx * tetCenter[0] + f.ny * tetCenter[1] + f.nz * tetCenter[2] - f.d
    if (dot > 1e-10) {
      // Normal points inward, flip
      const tmp = f.b; f.b = f.c; f.c = tmp
      const nn = computeHullFaceNormal(pts, f.a, f.b, f.c)
      f.nx = nn.nx; f.ny = nn.ny; f.nz = nn.nz; f.d = nn.d
    }
  }

  const usedPts = new Set([i0, i1, i2, i3])

  // Incremental convex hull
  for (let pi = 0; pi < pts.length; pi++) {
    if (usedPts.has(pi)) continue
    const pt = pts[pi]

    // Find visible faces
    const visible: number[] = []
    for (let fi = 0; fi < faces.length; fi++) {
      const f = faces[fi]
      const dist = f.nx * pt[0] + f.ny * pt[1] + f.nz * pt[2] - f.d
      if (dist > 1e-10) visible.push(fi)
    }

    if (visible.length === 0) continue // point is inside hull

    usedPts.add(pi)

    // Find horizon edges (boundary of visible faces)
    const horizonEdges: [number, number][] = []
    const visibleSet = new Set(visible)

    for (const fi of visible) {
      const f = faces[fi]
      const edges: [number, number][] = [[f.a, f.b], [f.b, f.c], [f.c, f.a]]
      for (const [ea, eb] of edges) {
        // Check if the adjacent face sharing this edge is NOT visible
        let isHorizon = true
        for (let fj = 0; fj < faces.length; fj++) {
          if (fj === fi || visibleSet.has(fj)) continue
          const of2 = faces[fj]
          const hasEdge = (of2.a === eb && of2.b === ea) || (of2.b === eb && of2.c === ea) || (of2.c === eb && of2.a === ea) ||
                         (of2.a === ea && of2.b === eb) || (of2.b === ea && of2.c === eb) || (of2.c === ea && of2.a === eb)
          if (hasEdge) { isHorizon = true; break }
        }
        if (isHorizon) {
          // Check this edge isn't shared with another visible face
          let sharedWithVisible = false
          for (const fj of visible) {
            if (fj === fi) continue
            const of2 = faces[fj]
            const hasEdge = (of2.a === eb && of2.b === ea) || (of2.b === eb && of2.c === ea) || (of2.c === eb && of2.a === ea) ||
                           (of2.a === ea && of2.b === eb) || (of2.b === ea && of2.c === eb) || (of2.c === ea && of2.a === eb)
            if (hasEdge) { sharedWithVisible = true; break }
          }
          if (!sharedWithVisible) {
            horizonEdges.push([ea, eb])
          }
        }
      }
    }

    // Remove visible faces (in reverse order to preserve indices)
    const sortedVisible = [...visible].sort((a, b) => b - a)
    for (const fi of sortedVisible) {
      faces.splice(fi, 1)
    }

    // Create new faces from horizon edges to the point
    for (const [ea, eb] of horizonEdges) {
      const nn = computeHullFaceNormal(pts, ea, eb, pi)
      const newFace: HullFace = { a: ea, b: eb, c: pi, ...nn }

      // Check normal points outward (away from hull center)
      // Compute approximate center of remaining hull
      let hcx = 0, hcy = 0, hcz = 0, hcount = 0
      for (const uid of usedPts) {
        hcx += pts[uid][0]; hcy += pts[uid][1]; hcz += pts[uid][2]; hcount++
      }
      if (hcount > 0) { hcx /= hcount; hcy /= hcount; hcz /= hcount }

      const dot = newFace.nx * hcx + newFace.ny * hcy + newFace.nz * hcz - newFace.d
      if (dot > 1e-10) {
        // Flip
        newFace.b = eb; newFace.a = ea;
        // Actually swap a and b
        const tmpA = newFace.a
        newFace.a = newFace.b
        newFace.b = tmpA
        const nn2 = computeHullFaceNormal(pts, newFace.a, newFace.b, newFace.c)
        newFace.nx = nn2.nx; newFace.ny = nn2.ny; newFace.nz = nn2.nz; newFace.d = nn2.d
      }

      faces.push(newFace)
    }
  }

  // Convert faces to mesh
  // Each face gets its own vertices (flat shading)
  let vi = 0
  for (const f of faces) {
    const pa = pts[f.a], pb = pts[f.b], pc = pts[f.c]
    v.push(pa[0], pa[1], pa[2], f.nx, f.ny, f.nz)
    v.push(pb[0], pb[1], pb[2], f.nx, f.ny, f.nz)
    v.push(pc[0], pc[1], pc[2], f.nx, f.ny, f.nz)
    ix.push(vi, vi + 1, vi + 2)
    vi += 3
  }

  return { v, ix }
}

export function makePolyhedron(points: number[][], faces: number[][]): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  if (points.length < 3 || faces.length < 1) return { v, ix }

  // Each face gets its own vertices (flat shading)
  let vi = 0
  for (const face of faces) {
    if (face.length < 3) continue
    // Compute face normal from first three vertices
    const p0 = points[face[0]] || [0,0,0]
    const p1 = points[face[1]] || [0,0,0]
    const p2 = points[face[2]] || [0,0,0]
    const e1x = p1[0] - p0[0], e1y = p1[1] - p0[1], e1z = p1[2] - p0[2]
    const e2x = p2[0] - p0[0], e2y = p2[1] - p0[1], e2z = p2[2] - p0[2]
    let nx = e1y * e2z - e1z * e2y
    let ny = e1z * e2x - e1x * e2z
    let nz = e1x * e2y - e1y * e2x
    const len = Math.sqrt(nx * nx + ny * ny + nz * nz)
    if (len > 1e-10) { nx /= len; ny /= len; nz /= len }

    // Emit vertices for each point in the face
    const baseVi = vi
    for (const idx of face) {
      const p = points[idx] || [0,0,0]
      v.push(p[0], p[1], p[2], nx, ny, nz)
      vi++
    }

    // Fan triangulation: anchor on the first vertex of the face
    for (let j = 1; j < face.length - 1; j++) {
      ix.push(baseVi, baseVi + j, baseVi + j + 1)
    }
  }
  return { v, ix }
}

/* ── Trapezoid / Pyramid / Donut generators ───────── */

export function makeTrapezoid(top: number, bottom: number, h: number, depth: number) {
  const v: number[] = [], ix: number[] = []
  const bt = bottom / 2, tt = top / 2, d = depth / 2

  // 8 vertices of the trapezoid prism
  // Bottom face (z=0): bl_fl, bl_fr, bl_br, bl_bl
  // Top face (z=h):    tl_fl, tl_fr, tl_br, tl_bl
  const verts = [
    [-bt, -d, 0], [ bt, -d, 0], [ bt,  d, 0], [-bt,  d, 0],  // bottom: 0,1,2,3
    [-tt, -d, h], [ tt, -d, h], [ tt,  d, h], [-tt,  d, h],   // top: 4,5,6,7
  ]

  // 6 faces: [indices, normal]
  // Bottom face (z=0) - normal (0,0,-1)
  const faces: { verts: number[][]; normal: [number, number, number] }[] = [
    { verts: [verts[0], verts[3], verts[2], verts[1]], normal: [0, 0, -1] },  // bottom
    { verts: [verts[4], verts[5], verts[6], verts[7]], normal: [0, 0, 1] },   // top
    { verts: [verts[0], verts[1], verts[5], verts[4]], normal: [0, -1, 0] },  // front (y=-d)
    { verts: [verts[2], verts[3], verts[7], verts[6]], normal: [0, 1, 0] },   // back (y=+d)
  ]

  // Left slope: from (-bt,_,0) to (-tt,_,h)
  // Normal for left slope: perpendicular to the slope surface
  const ldx = -tt - (-bt)  // = bt - tt (change in x going up)
  const ldz = h             // change in z going up
  // Normal to left slope points leftward: (-ldz, 0, ldx) normalized... actually:
  // The left face goes from bottom-left to top-left. The slope direction is (ldx, 0, ldz).
  // Face normal perpendicular to slope, pointing left: (-h, 0, bt - tt) -> but we need outward
  // Outward normal for left face: (-h, 0, -(bt - tt)) ... let's compute properly
  // Edge along slope: (-tt - (-bt), 0, h) = (bt-tt, 0, h)
  // Edge along depth: (0, depth, 0)
  // Normal = slope x depth = (0*h - depth*0, depth*(bt-tt) - 0*0, 0*0 - 0*(bt-tt))... no
  // slope = (bt-tt, 0, h), depthEdge = (0, 1, 0)
  // cross = (0*0 - h*1, h*0 - (bt-tt)*0, (bt-tt)*1 - 0*0) = (-h, 0, bt-tt)
  const lnx = -h, lny = 0, lnz = bt - tt
  const lnLen = Math.sqrt(lnx * lnx + lnz * lnz) || 1
  faces.push({
    verts: [verts[0], verts[4], verts[7], verts[3]],
    normal: [lnx / lnLen, lny / lnLen, lnz / lnLen]
  })

  // Right slope: from (bt,_,0) to (tt,_,h)
  // By symmetry, normal is (h, 0, bt-tt) normalized
  const rnx = h, rny = 0, rnz = bt - tt
  const rnLen = Math.sqrt(rnx * rnx + rnz * rnz) || 1
  faces.push({
    verts: [verts[1], verts[2], verts[6], verts[5]],
    normal: [rnx / rnLen, rny / rnLen, rnz / rnLen]
  })

  for (const face of faces) {
    const base = v.length / 6
    for (const vert of face.verts) {
      v.push(vert[0], vert[1], vert[2], face.normal[0], face.normal[1], face.normal[2])
    }
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

export function makePyramid(base: number, h: number, sides: number, center: boolean) {
  const v: number[] = [], ix: number[] = []
  const r = base / 2
  const zOff = center ? -h / 2 : 0

  // Generate base polygon vertices
  const baseVerts: [number, number][] = []
  for (let i = 0; i < sides; i++) {
    const angle = (2 * Math.PI * i) / sides
    baseVerts.push([r * Math.cos(angle), r * Math.sin(angle)])
  }

  const apex: [number, number, number] = [0, 0, h + zOff]

  // Bottom cap (normal pointing down, z = zOff)
  const bottomBase = v.length / 6
  for (let i = 0; i < sides; i++) {
    v.push(baseVerts[i][0], baseVerts[i][1], zOff, 0, 0, -1)
  }
  // Fan triangulation for bottom (winding order for downward normal)
  for (let i = 1; i < sides - 1; i++) {
    ix.push(bottomBase, bottomBase + i + 1, bottomBase + i)
  }

  // Side faces: N triangles
  for (let i = 0; i < sides; i++) {
    const i2 = (i + 1) % sides
    const p0: [number, number, number] = [baseVerts[i][0], baseVerts[i][1], zOff]
    const p1: [number, number, number] = [baseVerts[i2][0], baseVerts[i2][1], zOff]

    // Compute face normal
    const e1x = p1[0] - p0[0], e1y = p1[1] - p0[1], e1z = p1[2] - p0[2]
    const e2x = apex[0] - p0[0], e2y = apex[1] - p0[1], e2z = apex[2] - p0[2]
    let fnx = e1y * e2z - e1z * e2y
    let fny = e1z * e2x - e1x * e2z
    let fnz = e1x * e2y - e1y * e2x
    const fnLen = Math.sqrt(fnx * fnx + fny * fny + fnz * fnz) || 1
    fnx /= fnLen; fny /= fnLen; fnz /= fnLen

    const sBase = v.length / 6
    v.push(p0[0], p0[1], p0[2], fnx, fny, fnz)
    v.push(p1[0], p1[1], p1[2], fnx, fny, fnz)
    v.push(apex[0], apex[1], apex[2], fnx, fny, fnz)
    ix.push(sBase, sBase + 1, sBase + 2)
  }

  return { v, ix }
}

export function makeDonut(r1: number, r2: number, angle: number, fn: number) {
  if (angle >= 360) return makeTorus(r1, r2, fn)

  const v: number[] = [], ix: number[] = []
  const ringSegs = Math.max(4, Math.floor(fn * angle / 360))
  // Same r1<=0 / ratio guards as makeTorus.
  const tubeSegs = Math.max(8, Math.min(MAX_FN, r1 > 0 ? Math.floor(fn * r2 / r1) : 8))
  const angleRad = (angle * Math.PI) / 180

  // Generate torus surface vertices for partial sweep
  for (let i = 0; i <= ringSegs; i++) {
    const u = (angleRad * i) / ringSegs
    const cu = Math.cos(u), su = Math.sin(u)
    for (let j = 0; j <= tubeSegs; j++) {
      const vv = (2 * Math.PI * j) / tubeSegs
      const cv = Math.cos(vv), sv = Math.sin(vv)
      const x = (r1 + r2 * cv) * cu
      const y = r2 * sv
      const z = (r1 + r2 * cv) * su
      const nx = cv * cu
      const ny = sv
      const nz = cv * su
      v.push(x, y, z, nx, ny, nz)
    }
  }
  // Index the torus surface
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // End cap at angle=0 (ring index 0)
  // The cap center is at (r1, 0, 0) and the disc lies in the YZ plane locally
  // Normal for this cap points in the -Z direction of the sweep (tangent direction at start)
  // At u=0, tangent direction is (0, 0, 1) so cap normal is (0, 0, -1)
  const cap0Center = v.length / 6
  v.push(r1, 0, 0, 0, 0, -1)  // center of cap 0
  for (let j = 0; j <= tubeSegs; j++) {
    const vv = (2 * Math.PI * j) / tubeSegs
    const cv = Math.cos(vv), sv = Math.sin(vv)
    const x = r1 + r2 * cv
    const y = r2 * sv
    v.push(x, y, 0, 0, 0, -1)
  }
  for (let j = 0; j < tubeSegs; j++) {
    ix.push(cap0Center, cap0Center + 1 + j + 1, cap0Center + 1 + j)
  }

  // End cap at angle=angle
  const cu2 = Math.cos(angleRad), su2 = Math.sin(angleRad)
  // Cap center at (r1*cos(angle), 0, r1*sin(angle))
  // Tangent direction at end: (-sin(angle), 0, cos(angle)), so cap outward normal is same direction
  const capNx = -su2, capNy = 0, capNz = cu2  // outward-pointing tangent
  const cap1Center = v.length / 6
  v.push(r1 * cu2, 0, r1 * su2, capNx, capNy, capNz)
  for (let j = 0; j <= tubeSegs; j++) {
    const vv = (2 * Math.PI * j) / tubeSegs
    const cv = Math.cos(vv), sv = Math.sin(vv)
    const x = (r1 + r2 * cv) * cu2
    const y = r2 * sv
    const z = (r1 + r2 * cv) * su2
    v.push(x, y, z, capNx, capNy, capNz)
  }
  for (let j = 0; j < tubeSegs; j++) {
    ix.push(cap1Center, cap1Center + 1 + j, cap1Center + 1 + j + 1)
  }

  return { v, ix }
}

export function makeLattice(type: string, cell: number, r: number, size: [number, number, number], fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const [sx, sy, sz] = size
  const nx = Math.max(1, Math.floor(sx / cell))
  const ny = Math.max(1, Math.floor(sy / cell))
  const nz = Math.max(1, Math.floor(sz / cell))

  // Helper to merge a cylinder mesh with offset and optional rotation
  const addCyl = (cyl: { v: number[]; ix: number[] }, offsetX: number, offsetY: number, offsetZ: number, axis: 'x' | 'y' | 'z') => {
    const base = v.length / 6
    const cv = cyl.v
    const nv = cv.length / 6
    for (let i = 0; i < nv; i++) {
      let px = cv[i * 6], py = cv[i * 6 + 1], pz = cv[i * 6 + 2]
      let nnx = cv[i * 6 + 3], nny = cv[i * 6 + 4], nnz = cv[i * 6 + 5]
      // Rotate from Z-axis to target axis
      if (axis === 'x') {
        // Rotate 90 deg around Y: (x,y,z) -> (z,y,-x)
        const tmp = px; px = pz; pz = -tmp
        const tmpn = nnx; nnx = nnz; nnz = -tmpn
      } else if (axis === 'y') {
        // Rotate -90 deg around X: (x,y,z) -> (x,-z,y)
        const tmp = py; py = -pz; pz = tmp
        const tmpn = nny; nny = -nnz; nnz = tmpn
      }
      v.push(px + offsetX, py + offsetY, pz + offsetZ, nnx, nny, nnz)
    }
    for (const idx of cyl.ix) {
      ix.push(base + idx)
    }
  }

  void type // currently only cubic lattice

  // X-axis struts
  for (let iy = 0; iy <= ny; iy++) {
    for (let iz = 0; iz <= nz; iz++) {
      const cyl = makeCylinder(sx, r, r, false, fn)
      addCyl(cyl, 0, iy * cell, iz * cell, 'x')
    }
  }
  // Y-axis struts
  for (let ix2 = 0; ix2 <= nx; ix2++) {
    for (let iz = 0; iz <= nz; iz++) {
      const cyl = makeCylinder(sy, r, r, false, fn)
      addCyl(cyl, ix2 * cell, 0, iz * cell, 'y')
    }
  }
  // Z-axis struts
  for (let ix2 = 0; ix2 <= nx; ix2++) {
    for (let iy = 0; iy <= ny; iy++) {
      const cyl = makeCylinder(sz, r, r, false, fn)
      addCyl(cyl, ix2 * cell, iy * cell, 0, 'z')
    }
  }

  return { v, ix }
}

export function makeSlot(length: number, width: number, h: number, center: boolean): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const r = width / 2
  const halfLen = (length - width) / 2  // half of the straight section
  const fn = 16

  // Build 2D stadium profile
  const profile: number[][] = []
  // Right semicircle (center at x = halfLen)
  for (let i = 0; i <= fn / 2; i++) {
    const angle = -Math.PI / 2 + (Math.PI * i) / (fn / 2)
    profile.push([halfLen + r * Math.cos(angle), r * Math.sin(angle)])
  }
  // Left semicircle (center at x = -halfLen)
  for (let i = 0; i <= fn / 2; i++) {
    const angle = Math.PI / 2 + (Math.PI * i) / (fn / 2)
    profile.push([-halfLen + r * Math.cos(angle), r * Math.sin(angle)])
  }

  const np = profile.length
  const ox = center ? 0 : length / 2
  const oy = center ? 0 : width / 2
  const z0 = center ? -h / 2 : 0
  const z1 = center ? h / 2 : h

  // Shift profile to handle centering
  const shifted = profile.map(p => [p[0] + ox, p[1] + oy])

  // Bottom cap
  const baseBot = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(shifted[i][0], shifted[i][1], z0, 0, 0, -1)
  }
  const botTris = earClip(shifted)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap
  const baseTop = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(shifted[i][0], shifted[i][1], z1, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < np; i++) {
    const i2 = (i + 1) % np
    const x0 = shifted[i][0], y0 = shifted[i][1]
    const x1 = shifted[i2][0], y1 = shifted[i2][1]
    const ex = x1 - x0, ey = y1 - y0
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nnx = ey / len, nny = -ex / len
    const base = v.length / 6
    v.push(x0, y0, z0, nnx, nny, 0)
    v.push(x1, y1, z0, nnx, nny, 0)
    v.push(x1, y1, z1, nnx, nny, 0)
    v.push(x0, y0, z1, nnx, nny, 0)
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

export function makeCross(size: [number, number, number], arm: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const [sx, sy, sz] = size

  // Bar 1: along X, centered
  const bar1 = makeCube(sx, arm, sz, true)
  // Bar 2: along Y, centered
  const bar2 = makeCube(arm, sy, sz, true)

  // Merge bar1
  v.push(...bar1.v)
  ix.push(...bar1.ix)

  // Merge bar2 with offset
  const base = v.length / 6
  v.push(...bar2.v)
  for (const idx of bar2.ix) {
    ix.push(base + idx)
  }

  return { v, ix }
}

export function makeMaze(rows: number, cols: number, cell: number, wall: number, h: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []

  // Simple seeded random
  let seed = rows * 1000 + cols * 100 + cell * 10 + wall
  const rand = () => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff
    return seed / 0x7fffffff
  }

  // Initialize grid
  const visited: boolean[][] = []
  // walls: [right, bottom] for each cell
  const wallsR: boolean[][] = []
  const wallsB: boolean[][] = []
  for (let r = 0; r < rows; r++) {
    visited.push(new Array(cols).fill(false))
    wallsR.push(new Array(cols).fill(true))
    wallsB.push(new Array(cols).fill(true))
  }

  // Recursive backtracking maze generation
  const stack: [number, number][] = []
  const start: [number, number] = [0, 0]
  visited[start[0]][start[1]] = true
  stack.push(start)

  while (stack.length > 0) {
    const [cr, cc] = stack[stack.length - 1]
    // Find unvisited neighbors
    const neighbors: [number, number, string][] = []
    if (cr > 0 && !visited[cr - 1][cc]) neighbors.push([cr - 1, cc, 'up'])
    if (cr < rows - 1 && !visited[cr + 1][cc]) neighbors.push([cr + 1, cc, 'down'])
    if (cc > 0 && !visited[cr][cc - 1]) neighbors.push([cr, cc - 1, 'left'])
    if (cc < cols - 1 && !visited[cr][cc + 1]) neighbors.push([cr, cc + 1, 'right'])

    if (neighbors.length === 0) {
      stack.pop()
    } else {
      const choice = Math.floor(rand() * neighbors.length) % neighbors.length
      const [nr, nc, dir] = neighbors[choice]
      // Remove wall between current and chosen
      if (dir === 'right') wallsR[cr][cc] = false
      if (dir === 'left') wallsR[cr][nc] = false
      if (dir === 'down') wallsB[cr][cc] = false
      if (dir === 'up') wallsB[nr][cc] = false
      visited[nr][nc] = true
      stack.push([nr, nc])
    }
  }

  const addCube = (cx: number, cy: number, cz: number, csx: number, csy: number, csz: number) => {
    const cube = makeCube(csx, csy, csz, false)
    const base = v.length / 6
    const nv = cube.v.length / 6
    for (let i = 0; i < nv; i++) {
      v.push(cube.v[i * 6] + cx, cube.v[i * 6 + 1] + cy, cube.v[i * 6 + 2] + cz,
             cube.v[i * 6 + 3], cube.v[i * 6 + 4], cube.v[i * 6 + 5])
    }
    for (const idx of cube.ix) {
      ix.push(base + idx)
    }
  }

  // Floor
  const totalW = cols * cell + wall
  const totalH = rows * cell + wall
  addCube(0, 0, 0, totalW, totalH, wall)

  // Border walls
  // Bottom border (y=0)
  addCube(0, 0, 0, totalW, wall, h)
  // Top border (y=totalH-wall)
  addCube(0, totalH - wall, 0, totalW, wall, h)
  // Left border (x=0)
  addCube(0, 0, 0, wall, totalH, h)
  // Right border (x=totalW-wall)
  addCube(totalW - wall, 0, 0, wall, totalH, h)

  // Internal walls
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const cx = wall + c * cell
      const cy = wall + r * cell
      // Right wall
      if (wallsR[r][c] && c < cols - 1) {
        addCube(cx + cell - wall, cy, 0, wall, cell, h)
      }
      // Bottom wall (which is top in our Y direction)
      if (wallsB[r][c] && r < rows - 1) {
        addCube(cx, cy + cell - wall, 0, cell, wall, h)
      }
    }
  }

  return { v, ix }
}

export function makeFibonacciSphere(count: number, r: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const goldenAngle = Math.PI * (3 - Math.sqrt(5))
  const smallR = Math.max(0.3, r / Math.sqrt(count) * 0.5)

  for (let i = 0; i < count; i++) {
    const y = 1 - (2 * i) / (count - 1)  // y goes from 1 to -1
    const radiusAtY = Math.sqrt(1 - y * y)
    const theta = goldenAngle * i

    const px = r * radiusAtY * Math.cos(theta)
    const py = r * y
    const pz = r * radiusAtY * Math.sin(theta)

    // Add a small sphere at this position
    const sphere = makeSphere(smallR, Math.max(4, Math.floor(fn / 4)))
    const base = v.length / 6
    const nv = sphere.v.length / 6
    for (let j = 0; j < nv; j++) {
      v.push(sphere.v[j * 6] + px, sphere.v[j * 6 + 1] + py, sphere.v[j * 6 + 2] + pz,
             sphere.v[j * 6 + 3], sphere.v[j * 6 + 4], sphere.v[j * 6 + 5])
    }
    for (const idx of sphere.ix) {
      ix.push(base + idx)
    }
  }

  return { v, ix }
}

export function makeEllipsoid(rx: number, ry: number, rz: number, fn: number): { v: number[]; ix: number[] } {
  const sphere = makeSphere(1, fn)
  const v: number[] = []
  const nv = sphere.v.length / 6
  for (let i = 0; i < nv; i++) {
    const px = sphere.v[i * 6] * rx
    const py = sphere.v[i * 6 + 1] * ry
    const pz = sphere.v[i * 6 + 2] * rz
    // Recompute normal for ellipsoid: gradient of (x/rx)^2 + (y/ry)^2 + (z/rz)^2
    let nnx = px / (rx * rx)
    let nny = py / (ry * ry)
    let nnz = pz / (rz * rz)
    const nlen = Math.sqrt(nnx * nnx + nny * nny + nnz * nnz) || 1
    nnx /= nlen; nny /= nlen; nnz /= nlen
    v.push(px, py, pz, nnx, nny, nnz)
  }
  return { v, ix: sphere.ix }
}

export function makeHemisphere(r: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const halfSeg = Math.floor(fn / 2)

  // Upper hemisphere: phi from 0 (top) to PI/2 (equator)
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = (Math.PI / 2) * ri / halfSeg
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = cp, nz = sp * Math.sin(th)
      v.push(r * nx, r * ny, r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Flat circular cap at y=0 (equator)
  const capCenter = v.length / 6
  v.push(0, 0, 0, 0, -1, 0)  // center, normal pointing down
  for (let si = 0; si <= fn; si++) {
    const th = 2 * Math.PI * si / fn
    v.push(r * Math.cos(th), 0, r * Math.sin(th), 0, -1, 0)
  }
  for (let si = 0; si < fn; si++) {
    ix.push(capCenter, capCenter + 1 + si + 1, capCenter + 1 + si)
  }

  return { v, ix }
}

export function makeOgive(r: number, h: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  // Ogive radius of curvature: R = (r^2 + h^2) / (2*r)
  const R = (r * r + h * h) / (2 * r)
  const stacks = Math.max(4, Math.floor(fn / 2))

  // Generate profile: z goes from 0 (base) to h (tip)
  for (let ri = 0; ri <= stacks; ri++) {
    const z = h * ri / stacks
    // Ogive profile: x(z) = sqrt(R^2 - (z - h)^2) - (R - r)
    const inner = R * R - (z - h) * (z - h)
    const xr = inner > 0 ? Math.sqrt(inner) - (R - r) : 0
    const radius = Math.max(0, xr)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx0 = Math.cos(th), nz0 = Math.sin(th)
      const px = radius * nx0, py = z, pz = radius * nz0
      // Approximate normal: tangent along profile gives slope
      const zUp = h * Math.min(ri + 1, stacks) / stacks
      const innerUp = R * R - (zUp - h) * (zUp - h)
      const xrUp = innerUp > 0 ? Math.sqrt(innerUp) - (R - r) : 0
      const zDn = h * Math.max(ri - 1, 0) / stacks
      const innerDn = R * R - (zDn - h) * (zDn - h)
      const xrDn = innerDn > 0 ? Math.sqrt(innerDn) - (R - r) : 0
      const dr = (xrUp - xrDn) / (zUp - zDn || 1)
      // Normal perpendicular to surface: (1, -dr, 0) rotated around Y
      let nnx = nx0, nny = -dr, nnz = nz0
      const nlen = Math.sqrt(nnx * nnx + nny * nny + nnz * nnz) || 1
      nnx /= nlen; nny /= nlen; nnz /= nlen
      v.push(px, py, pz, nnx, nny, nnz)
    }
  }
  for (let ri = 0; ri < stacks; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Bottom cap at z=0
  const capCenter = v.length / 6
  v.push(0, 0, 0, 0, -1, 0)
  for (let si = 0; si <= fn; si++) {
    const th = 2 * Math.PI * si / fn
    v.push(r * Math.cos(th), 0, r * Math.sin(th), 0, -1, 0)
  }
  for (let si = 0; si < fn; si++) {
    ix.push(capCenter, capCenter + 1 + si + 1, capCenter + 1 + si)
  }

  return { v, ix }
}

export function makeTeardrop(r: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const halfSeg = Math.max(4, Math.floor(fn / 2))

  // Lower hemisphere: phi from PI/2 (equator, y=0) to PI (bottom, y=-r)
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = Math.PI / 2 + (Math.PI / 2) * ri / halfSeg
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = cp, nz = sp * Math.sin(th)
      v.push(r * nx, r * ny, r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // 45-degree cone from equator (y=0) to tip (y=r)
  // At equator radius = r, at tip radius = 0, height = r (45 degrees)
  const coneStacks = halfSeg
  const baseOffset = v.length / 6
  for (let ri = 0; ri <= coneStacks; ri++) {
    const t = ri / coneStacks
    const y = r * t
    const radius = r * (1 - t)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const cx = Math.cos(th), cz = Math.sin(th)
      const px = radius * cx, py = y, pz = radius * cz
      // Cone normal: 45-degree slope, normalized
      const s45 = Math.SQRT1_2
      const nnx = s45 * cx, nny = s45, nnz = s45 * cz
      v.push(px, py, pz, nnx, nny, nnz)
    }
  }
  for (let ri = 0; ri < coneStacks; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = baseOffset + ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Stitch hemisphere equator (row 0) to cone base (row 0)
  // They share the same y=0, radius=r ring, so just connect them
  for (let si = 0; si < fn; si++) {
    const hTop = si  // hemisphere row 0 (equator)
    const cBot = baseOffset + si  // cone row 0 (equator)
    ix.push(hTop, cBot, hTop + 1)
    ix.push(hTop + 1, cBot, cBot + 1)
  }

  return { v, ix }
}

/* ── Main evaluator ───────────────────────────────── */
