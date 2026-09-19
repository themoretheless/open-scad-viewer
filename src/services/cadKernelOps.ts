import { tessellateBrepGear, type BrepGearSpec } from './geometry/brep'
import Module, {
  type CrossSection,
  type ErrorStatus,
  type CadSolid,
  type CadToplevel,
  type Mat3,
  type Mat4,
  type Polygons,
  type Vec2,
  type Vec3,
} from './geometry/module'

const CAD_KERNEL_HANDLE = Symbol('CadKernelHandle')

/** Opaque outside this narrow WASM adapter. */
export interface CadKernelHandle {
  readonly dimension: 2 | 3
  readonly [CAD_KERNEL_HANDLE]: true
}

interface OwnedCadKernelHandle extends CadKernelHandle {
  readonly geometry: CadSolid | CrossSection
  deleted: boolean
}

export interface CadKernelMesh {
  readonly numProp: number
  readonly numTri: number
  readonly numVert: number
  readonly vertProperties: Float32Array
  readonly triVerts: Uint32Array
  readonly mergeFromVert: Uint32Array
  readonly mergeToVert: Uint32Array
  readonly runIndex: Uint32Array
  readonly runOriginalID: Uint32Array
  readonly runFlags: Uint8Array
  readonly faceID: Uint32Array
}

export interface CadKernelSolidAnalysis {
  readonly volume: number
  readonly surfaceArea: number
  readonly mesh: CadKernelMesh
}

export interface CadKernelOps {
  readonly implementationKey: 'own-rust-cad-plan-v1'
  empty2(): CadKernelHandle
  empty3(): CadKernelHandle
  box(size: readonly [number, number, number], center: boolean): CadKernelHandle
  sphere(radius: number, radialSegments: number): CadKernelHandle
  cylinder(
    height: number,
    radiusBottom: number,
    radiusTop: number,
    radialSegments: number,
    center: boolean,
  ): CadKernelHandle
  polyhedron(
    vertices: readonly (readonly [number, number, number])[],
    triangles: readonly (readonly [number, number, number])[],
  ): CadKernelHandle
  rectangle(size: readonly [number, number], center: boolean): CadKernelHandle
  circle(radius: number, radialSegments: number): CadKernelHandle
  polygon(
    rings: readonly (readonly (readonly [number, number])[])[],
    fillRule?: 'EvenOdd' | 'NonZero',
  ): CadKernelHandle
  ofMesh(vertProperties: Float32Array, triVerts: Uint32Array): CadKernelHandle
  /** Involute gear: the exact NURBS body tessellated at `segments` per edge. */
  gear(spec: BrepGearSpec, segments: number): CadKernelHandle
  translate(input: CadKernelHandle, offset: readonly number[]): CadKernelHandle
  scale(input: CadKernelHandle, factors: readonly number[]): CadKernelHandle
  rotate(input: CadKernelHandle, angles: number | readonly number[]): CadKernelHandle
  mirror(input: CadKernelHandle, normal: readonly number[]): CadKernelHandle
  transform2(input: CadKernelHandle, matrix: readonly number[]): CadKernelHandle
  transform3(input: CadKernelHandle, matrix: readonly number[]): CadKernelHandle
  boolean2(
    operation: 'union' | 'intersection' | 'difference',
    inputs: readonly CadKernelHandle[],
  ): CadKernelHandle
  boolean3(
    operation: 'union' | 'intersection' | 'difference',
    inputs: readonly CadKernelHandle[],
  ): CadKernelHandle
  hull2(inputs: readonly CadKernelHandle[]): CadKernelHandle
  hull3(inputs: readonly CadKernelHandle[]): CadKernelHandle
  linearExtrude(
    input: CadKernelHandle,
    height: number,
    slices: number,
    twistDegrees: number,
    scale: readonly [number, number],
    center: boolean,
  ): CadKernelHandle
  rotateExtrude(
    input: CadKernelHandle,
    radialSegments: number,
    angleDegrees: number,
  ): CadKernelHandle
  projection(input: CadKernelHandle, cut: boolean): CadKernelHandle
  offset(
    input: CadKernelHandle,
    distance: number,
    join?: 'Square' | 'Round' | 'Miter',
    miterLimit?: number,
    circularSegments?: number,
  ): CadKernelHandle
  minkowskiSum3(left: CadKernelHandle, right: CadKernelHandle): CadKernelHandle
  polygons(input: CadKernelHandle): Array<Array<[number, number]>>
  bounds(input: CadKernelHandle): { min: number[]; max: number[] }
  firstVertex3(input: CadKernelHandle): [number, number, number] | null
  mirrorZ(input: CadKernelHandle): CadKernelHandle
  translateZ(input: CadKernelHandle, distance: number): CadKernelHandle
  isEmpty(input: CadKernelHandle): boolean
  originalId(input: CadKernelHandle): number | null
  asOriginal(input: CadKernelHandle): CadKernelHandle
  analyzeSolid(input: CadKernelHandle): CadKernelSolidAnalysis
  delete(input: CadKernelHandle): void
}

function ownedHandle(
  value: CadKernelHandle,
  expectedDimension?: 2 | 3,
): OwnedCadKernelHandle {
  const handle = value as Partial<OwnedCadKernelHandle>
  if (handle[CAD_KERNEL_HANDLE] !== true
    || handle.geometry === undefined
    || handle.deleted !== false) {
    throw new TypeError('Foreign or deleted CAD kernel handle')
  }
  if (expectedDimension !== undefined && handle.dimension !== expectedDimension) {
    throw new TypeError(`Expected a ${expectedDimension}D CAD kernel handle`)
  }
  return handle as OwnedCadKernelHandle
}

function handle(dimension: 2 | 3, geometry: CadSolid | CrossSection): CadKernelHandle {
  return {
    dimension,
    geometry,
    deleted: false,
    [CAD_KERNEL_HANDLE]: true as const,
  } as OwnedCadKernelHandle
}

function cadStatusError(wasm: CadToplevel, status: ErrorStatus): Error {
  if (wasm.CadError !== undefined) return new wasm.CadError(status)
  const messages: Readonly<Partial<Record<ErrorStatus, string>>> = {
    NonFiniteVertex: 'Non-finite vertex',
    NotManifold: 'Not manifold',
    VertexOutOfBounds: 'Vertex index out of bounds',
    PropertiesWrongLength: 'Properties have wrong length',
    MissingPositionProperties: 'Less than three properties',
    MergeVectorsDifferentLengths: 'Merge vectors have different lengths',
    MergeIndexOutOfBounds: 'Merge index out of bounds',
    TransformWrongLength: 'Transform vector has wrong length',
    RunIndexWrongLength: 'Run index vector has wrong length',
    FaceIDWrongLength: 'Face ID vector has wrong length',
    InvalidConstruction: 'Solid constructed with invalid parameters',
    ResultTooLarge: 'Result exceeds maximum size',
    InvalidTangents: 'Invalid halfedge tangents',
    Cancelled: 'Cancelled',
  }
  const error = new Error(messages[status] ?? 'Unknown error')
  error.name = 'CadError'
  return error
}

function vec2(value: readonly [number, number]): Vec2 {
  return [...value]
}

function vec3(value: readonly [number, number, number]): Vec3 {
  return [...value]
}

function matrix3(value: readonly number[]): Mat3 {
  return [...value] as Mat3
}

function matrix4(value: readonly number[]): Mat4 {
  return [...value] as Mat4
}

export function createCadKernelOps(
  wasm: CadToplevel,
  rawCadSolidConstructor: typeof CadSolid,
): CadKernelOps {
  const geometry2 = (value: CadKernelHandle): CrossSection => (
    ownedHandle(value, 2).geometry as CrossSection
  )
  const geometry3 = (value: CadKernelHandle): CadSolid => (
    ownedHandle(value, 3).geometry as CadSolid
  )
  const geometries2 = (values: readonly CadKernelHandle[]) => values.map(geometry2)
  const geometries3 = (values: readonly CadKernelHandle[]) => values.map(geometry3)

  const ops: CadKernelOps = {
    implementationKey: 'own-rust-cad-plan-v1' as const,
    empty2() {
      return handle(2, wasm.CrossSection.square([0, 0]))
    },
    empty3() {
      return handle(3, wasm.CadSolid.union([]))
    },
    box(size, center) {
      return handle(3, wasm.CadSolid.cube(vec3(size), center))
    },
    sphere(radius, radialSegments) {
      return handle(3, wasm.CadSolid.sphere(radius, radialSegments))
    },
    cylinder(height, radiusBottom, radiusTop, radialSegments, center) {
      return handle(3, wasm.CadSolid.cylinder(
        height, radiusBottom, radiusTop, radialSegments, center,
      ))
    },
    polyhedron(vertices, triangles) {
      const vertexData = new Float32Array(vertices.length * 3)
      vertices.forEach((vertex, index) => vertexData.set(vertex, index * 3))
      const triangleData = new Uint32Array(triangles.length * 3)
      triangles.forEach((triangle, index) => triangleData.set(triangle, index * 3))
      const mesh = new wasm.Mesh({
        numProp: 3,
        vertProperties: vertexData,
        triVerts: triangleData,
      })
      mesh.merge()
      // setup()'s convenience `ofMesh` wrapper throws after allocating an
      // invalid native solid and cannot delete that unreachable object.
      // Construct through the captured raw class so every non-NoError status
      // is explicitly destroyed before the compatible error is raised.
      let geometry: CadSolid | undefined
      try {
        geometry = new rawCadSolidConstructor(mesh)
        const status = geometry.status()
        if (status !== 'NoError') {
          const failure = cadStatusError(wasm, status)
          const invalid = geometry
          geometry = undefined
          try {
            invalid.delete()
          } catch (cleanupError) {
            throw new AggregateError([failure, cleanupError], 'Invalid solid cleanup failed')
          }
          throw failure
        }
        const result = handle(3, geometry)
        geometry = undefined
        return result
      } catch (error) {
        if (geometry !== undefined) {
          try {
            geometry.delete()
          } catch (cleanupError) {
            throw new AggregateError([error, cleanupError], 'Solid construction and cleanup failed')
          }
        }
        throw error
      }
    },
    rectangle(size, center) {
      return handle(2, wasm.CrossSection.square(vec2(size), center))
    },
    circle(radius, radialSegments) {
      return handle(2, wasm.CrossSection.circle(radius, radialSegments))
    },
    polygon(rings, fillRule = 'EvenOdd') {
      const polygons = rings.map(ring => ring.map(point => vec2(point))) as Polygons
      return handle(2, wasm.CrossSection.ofPolygons(polygons, fillRule))
    },
    gear(spec, segments) {
      const built = tessellateBrepGear(spec, Math.min(32, Math.max(1, Math.round(segments))))
      return ops.ofMesh(new Float32Array(built.positions), new Uint32Array(built.indices))
    },
    ofMesh(vertProperties, triVerts) {
      const mesh = new wasm.Mesh({
        numProp: 3,
        vertProperties,
        triVerts,
      })
      mesh.merge()
      let geometry: CadSolid | undefined
      try {
        geometry = new rawCadSolidConstructor(mesh)
        const status = geometry.status()
        if (status !== 'NoError') {
          const failure = cadStatusError(wasm, status)
          const invalid = geometry
          geometry = undefined
          try {
            invalid.delete()
          } catch (cleanupError) {
            throw new AggregateError([failure, cleanupError], 'Invalid solid cleanup failed')
          }
          throw failure
        }
        const result = handle(3, geometry)
        geometry = undefined
        return result
      } catch (error) {
        if (geometry !== undefined) {
          try {
            geometry.delete()
          } catch (cleanupError) {
            throw new AggregateError([error, cleanupError], 'Solid construction and cleanup failed')
          }
        }
        throw error
      }
    },
    translate(input, offset) {
      if (input.dimension === 2) {
        return handle(2, geometry2(input).translate([offset[0] ?? 0, offset[1] ?? 0]))
      }
      return handle(3, geometry3(input).translate([offset[0] ?? 0, offset[1] ?? 0, offset[2] ?? 0]))
    },
    scale(input, factors) {
      if (input.dimension === 2) {
        return handle(2, geometry2(input).scale([factors[0] ?? 1, factors[1] ?? factors[0] ?? 1]))
      }
      return handle(3, geometry3(input).scale([
        factors[0] ?? 1,
        factors[1] ?? factors[0] ?? 1,
        factors[2] ?? factors[0] ?? 1,
      ]))
    },
    rotate(input, angles) {
      if (input.dimension === 2) {
        const degrees = typeof angles === 'number' ? angles : angles[2] ?? 0
        return handle(2, geometry2(input).rotate(degrees))
      }
      const vector = typeof angles === 'number' ? [0, 0, angles] : angles
      return handle(3, geometry3(input).rotate([vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]))
    },
    mirror(input, normal) {
      if (input.dimension === 2) {
        return handle(2, geometry2(input).mirror([normal[0] ?? 0, normal[1] ?? 0]))
      }
      return handle(3, geometry3(input).mirror([normal[0] ?? 0, normal[1] ?? 0, normal[2] ?? 0]))
    },
    transform2(input, matrix) {
      return handle(2, geometry2(input).transform(matrix3(matrix)))
    },
    transform3(input, matrix) {
      return handle(3, geometry3(input).transform(matrix4(matrix)))
    },
    boolean2(operation, inputs) {
      const sections = geometries2(inputs)
      if (operation === 'union') return handle(2, wasm.CrossSection.union(sections))
      if (operation === 'intersection') return handle(2, wasm.CrossSection.intersection(sections))
      return handle(2, wasm.CrossSection.difference(sections))
    },
    boolean3(operation, inputs) {
      const solids = geometries3(inputs)
      if (operation === 'union') return handle(3, wasm.CadSolid.union(solids))
      if (operation === 'intersection') return handle(3, wasm.CadSolid.intersection(solids))
      return handle(3, wasm.CadSolid.difference(solids))
    },
    hull2(inputs) {
      return handle(2, wasm.CrossSection.hull(geometries2(inputs)))
    },
    hull3(inputs) {
      return handle(3, wasm.CadSolid.hull(geometries3(inputs)))
    },
    linearExtrude(input, height, slices, twistDegrees, scale, center) {
      return handle(3, wasm.CadSolid.extrude(
        geometry2(input), height, slices, twistDegrees, vec2(scale), center,
      ))
    },
    rotateExtrude(input, radialSegments, angleDegrees) {
      return handle(3, wasm.CadSolid.revolve(
        geometry2(input), radialSegments, angleDegrees,
      ))
    },
    projection(input, cut) {
      const solid = geometry3(input)
      return handle(2, cut ? solid.slice(0) : solid.project())
    },
    offset(input, distance, join, miterLimit, circularSegments) {
      if (join === undefined) return handle(2, geometry2(input).offset(distance))
      return handle(2, geometry2(input).offset(distance, join, miterLimit, circularSegments))
    },
    minkowskiSum3(left, right) {
      return handle(3, geometry3(left).minkowskiSum(geometry3(right)))
    },
    polygons(input) {
      return geometry2(input).toPolygons().map(ring => ring.map(point => [point[0], point[1]] as [number, number]))
    },
    bounds(input) {
      if (input.dimension === 2) {
        const box = geometry2(input).bounds()
        return { min: [...box.min], max: [...box.max] }
      }
      const box = geometry3(input).boundingBox()
      return { min: [...box.min], max: [...box.max] }
    },
    firstVertex3(input) {
      const mesh = geometry3(input).getMesh()
      if (mesh.numVert === 0) return null
      const vertex = mesh.position(0)
      return [vertex[0], vertex[1], vertex[2]]
    },
    mirrorZ(input) {
      return handle(3, geometry3(input).mirror([0, 0, 1]))
    },
    translateZ(input, distance) {
      return handle(3, geometry3(input).translate([0, 0, distance]))
    },
    isEmpty(input) {
      return ownedHandle(input).geometry.isEmpty()
    },
    originalId(input) {
      return input.dimension === 3 ? geometry3(input).originalID() : null
    },
    asOriginal(input) {
      return handle(3, geometry3(input).asOriginal())
    },
    analyzeSolid(input) {
      const solid = geometry3(input)
      const volume = solid.volume()
      const surfaceArea = solid.surfaceArea()
      const normalized = handle(3, solid.calculateNormals(0, 52.5))
      try {
        // getMesh() returns arrays the kernel copied out for this call only;
        // they are owned here and may be published or transferred as they are.
        const mesh = geometry3(normalized).getMesh()
        return Object.freeze({
          volume,
          surfaceArea,
          mesh: Object.freeze({
            numProp: mesh.numProp,
            numTri: mesh.numTri,
            numVert: mesh.numVert,
            vertProperties: mesh.vertProperties,
            triVerts: mesh.triVerts,
            mergeFromVert: mesh.mergeFromVert,
            mergeToVert: mesh.mergeToVert,
            runIndex: mesh.runIndex,
            runOriginalID: mesh.runOriginalID,
            runFlags: mesh.runFlags,
            faceID: mesh.faceID,
          }),
        })
      } finally {
        const owned = ownedHandle(normalized, 3)
        owned.deleted = true
        owned.geometry.delete()
      }
    },
    delete(input) {
      const owned = ownedHandle(input)
      owned.deleted = true
      owned.geometry.delete()
    },
  }
  return Object.freeze(ops)
}

/** Each load creates an uninstrumented module with no process-global GC list. */
export async function loadCadKernelOps(): Promise<CadKernelOps> {
  const module = await Module()
  const rawCadSolidConstructor = module.CadSolid
  module.setup()
  return createCadKernelOps(module, rawCadSolidConstructor)
}
