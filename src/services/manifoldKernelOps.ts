import Module, {
  type CrossSection,
  type ErrorStatus,
  type Manifold,
  type ManifoldToplevel,
  type Mat3,
  type Mat4,
  type Polygons,
  type Vec2,
  type Vec3,
} from './geometry/module'

const MANIFOLD_KERNEL_HANDLE = Symbol('ManifoldKernelHandle')

/** Opaque outside this narrow WASM adapter. */
export interface ManifoldKernelHandle {
  readonly dimension: 2 | 3
  readonly [MANIFOLD_KERNEL_HANDLE]: true
}

interface OwnedManifoldKernelHandle extends ManifoldKernelHandle {
  readonly geometry: Manifold | CrossSection
  deleted: boolean
}

export interface ManifoldKernelMesh {
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

export interface ManifoldKernelSolidAnalysis {
  readonly volume: number
  readonly surfaceArea: number
  readonly mesh: ManifoldKernelMesh
}

export interface ManifoldKernelOps {
  readonly implementationKey: 'own-rust-cad-plan-v1'
  empty2(): ManifoldKernelHandle
  empty3(): ManifoldKernelHandle
  box(size: readonly [number, number, number], center: boolean): ManifoldKernelHandle
  sphere(radius: number, radialSegments: number): ManifoldKernelHandle
  cylinder(
    height: number,
    radiusBottom: number,
    radiusTop: number,
    radialSegments: number,
    center: boolean,
  ): ManifoldKernelHandle
  polyhedron(
    vertices: readonly (readonly [number, number, number])[],
    triangles: readonly (readonly [number, number, number])[],
  ): ManifoldKernelHandle
  rectangle(size: readonly [number, number], center: boolean): ManifoldKernelHandle
  circle(radius: number, radialSegments: number): ManifoldKernelHandle
  polygon(
    rings: readonly (readonly (readonly [number, number])[])[],
    fillRule?: 'EvenOdd' | 'NonZero',
  ): ManifoldKernelHandle
  ofMesh(vertProperties: Float32Array, triVerts: Uint32Array): ManifoldKernelHandle
  translate(input: ManifoldKernelHandle, offset: readonly number[]): ManifoldKernelHandle
  scale(input: ManifoldKernelHandle, factors: readonly number[]): ManifoldKernelHandle
  rotate(input: ManifoldKernelHandle, angles: number | readonly number[]): ManifoldKernelHandle
  mirror(input: ManifoldKernelHandle, normal: readonly number[]): ManifoldKernelHandle
  transform2(input: ManifoldKernelHandle, matrix: readonly number[]): ManifoldKernelHandle
  transform3(input: ManifoldKernelHandle, matrix: readonly number[]): ManifoldKernelHandle
  boolean2(
    operation: 'union' | 'intersection' | 'difference',
    inputs: readonly ManifoldKernelHandle[],
  ): ManifoldKernelHandle
  boolean3(
    operation: 'union' | 'intersection' | 'difference',
    inputs: readonly ManifoldKernelHandle[],
  ): ManifoldKernelHandle
  hull2(inputs: readonly ManifoldKernelHandle[]): ManifoldKernelHandle
  hull3(inputs: readonly ManifoldKernelHandle[]): ManifoldKernelHandle
  linearExtrude(
    input: ManifoldKernelHandle,
    height: number,
    slices: number,
    twistDegrees: number,
    scale: readonly [number, number],
    center: boolean,
  ): ManifoldKernelHandle
  rotateExtrude(
    input: ManifoldKernelHandle,
    radialSegments: number,
    angleDegrees: number,
  ): ManifoldKernelHandle
  projection(input: ManifoldKernelHandle, cut: boolean): ManifoldKernelHandle
  offset(
    input: ManifoldKernelHandle,
    distance: number,
    join?: 'Square' | 'Round' | 'Miter',
    miterLimit?: number,
    circularSegments?: number,
  ): ManifoldKernelHandle
  minkowskiSum3(left: ManifoldKernelHandle, right: ManifoldKernelHandle): ManifoldKernelHandle
  polygons(input: ManifoldKernelHandle): Array<Array<[number, number]>>
  bounds(input: ManifoldKernelHandle): { min: number[]; max: number[] }
  firstVertex3(input: ManifoldKernelHandle): [number, number, number] | null
  mirrorZ(input: ManifoldKernelHandle): ManifoldKernelHandle
  translateZ(input: ManifoldKernelHandle, distance: number): ManifoldKernelHandle
  isEmpty(input: ManifoldKernelHandle): boolean
  originalId(input: ManifoldKernelHandle): number | null
  asOriginal(input: ManifoldKernelHandle): ManifoldKernelHandle
  analyzeSolid(input: ManifoldKernelHandle): ManifoldKernelSolidAnalysis
  delete(input: ManifoldKernelHandle): void
}

function ownedHandle(
  value: ManifoldKernelHandle,
  expectedDimension?: 2 | 3,
): OwnedManifoldKernelHandle {
  const handle = value as Partial<OwnedManifoldKernelHandle>
  if (handle[MANIFOLD_KERNEL_HANDLE] !== true
    || handle.geometry === undefined
    || handle.deleted !== false) {
    throw new TypeError('Foreign or deleted Manifold kernel handle')
  }
  if (expectedDimension !== undefined && handle.dimension !== expectedDimension) {
    throw new TypeError(`Expected a ${expectedDimension}D Manifold kernel handle`)
  }
  return handle as OwnedManifoldKernelHandle
}

function handle(dimension: 2 | 3, geometry: Manifold | CrossSection): ManifoldKernelHandle {
  return {
    dimension,
    geometry,
    deleted: false,
    [MANIFOLD_KERNEL_HANDLE]: true as const,
  } as OwnedManifoldKernelHandle
}

function manifoldStatusError(wasm: ManifoldToplevel, status: ErrorStatus): Error {
  const constructor = (wasm as unknown as {
    ManifoldError?: new (code: ErrorStatus) => Error
  }).ManifoldError
  if (constructor !== undefined) return new constructor(status)
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
    InvalidConstruction: 'Manifold constructed with invalid parameters',
    ResultTooLarge: 'Result exceeds maximum size',
    InvalidTangents: 'Invalid halfedge tangents',
    Cancelled: 'Cancelled',
  }
  const error = new Error(messages[status] ?? 'Unknown error')
  error.name = 'ManifoldError'
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

export function createManifoldKernelOps(
  wasm: ManifoldToplevel,
  rawManifoldConstructor: typeof Manifold,
): ManifoldKernelOps {
  const geometry2 = (value: ManifoldKernelHandle): CrossSection => (
    ownedHandle(value, 2).geometry as CrossSection
  )
  const geometry3 = (value: ManifoldKernelHandle): Manifold => (
    ownedHandle(value, 3).geometry as Manifold
  )
  const geometries2 = (values: readonly ManifoldKernelHandle[]) => values.map(geometry2)
  const geometries3 = (values: readonly ManifoldKernelHandle[]) => values.map(geometry3)

  const ops: ManifoldKernelOps = {
    implementationKey: 'own-rust-cad-plan-v1' as const,
    empty2() {
      return handle(2, wasm.CrossSection.square([0, 0]))
    },
    empty3() {
      return handle(3, wasm.Manifold.union([]))
    },
    box(size, center) {
      return handle(3, wasm.Manifold.cube(vec3(size), center))
    },
    sphere(radius, radialSegments) {
      return handle(3, wasm.Manifold.sphere(radius, radialSegments))
    },
    cylinder(height, radiusBottom, radiusTop, radialSegments, center) {
      return handle(3, wasm.Manifold.cylinder(
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
      // invalid native Manifold and cannot delete that unreachable object.
      // Construct through the captured raw class so every non-NoError status
      // is explicitly destroyed before the compatible error is raised.
      let geometry: Manifold | undefined
      try {
        geometry = new rawManifoldConstructor(mesh)
        const status = geometry.status()
        if (status !== 'NoError') {
          const failure = manifoldStatusError(wasm, status)
          const invalid = geometry
          geometry = undefined
          try {
            invalid.delete()
          } catch (cleanupError) {
            throw new AggregateError([failure, cleanupError], 'Invalid Manifold cleanup failed')
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
            throw new AggregateError([error, cleanupError], 'Manifold construction and cleanup failed')
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
    ofMesh(vertProperties, triVerts) {
      const mesh = new wasm.Mesh({
        numProp: 3,
        vertProperties,
        triVerts,
      })
      mesh.merge()
      let geometry: Manifold | undefined
      try {
        geometry = new rawManifoldConstructor(mesh)
        const status = geometry.status()
        if (status !== 'NoError') {
          const failure = manifoldStatusError(wasm, status)
          const invalid = geometry
          geometry = undefined
          try {
            invalid.delete()
          } catch (cleanupError) {
            throw new AggregateError([failure, cleanupError], 'Invalid Manifold cleanup failed')
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
            throw new AggregateError([error, cleanupError], 'Manifold construction and cleanup failed')
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
      if (operation === 'union') return handle(3, wasm.Manifold.union(solids))
      if (operation === 'intersection') return handle(3, wasm.Manifold.intersection(solids))
      return handle(3, wasm.Manifold.difference(solids))
    },
    hull2(inputs) {
      return handle(2, wasm.CrossSection.hull(geometries2(inputs)))
    },
    hull3(inputs) {
      return handle(3, wasm.Manifold.hull(geometries3(inputs)))
    },
    linearExtrude(input, height, slices, twistDegrees, scale, center) {
      return handle(3, wasm.Manifold.extrude(
        geometry2(input), height, slices, twistDegrees, vec2(scale), center,
      ))
    },
    rotateExtrude(input, radialSegments, angleDegrees) {
      return handle(3, wasm.Manifold.revolve(
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
        const mesh = geometry3(normalized).getMesh()
        return Object.freeze({
          volume,
          surfaceArea,
          mesh: Object.freeze({
            numProp: mesh.numProp,
            numTri: mesh.numTri,
            numVert: mesh.numVert,
            vertProperties: new Float32Array(mesh.vertProperties),
            triVerts: new Uint32Array(mesh.triVerts),
            mergeFromVert: new Uint32Array(mesh.mergeFromVert),
            mergeToVert: new Uint32Array(mesh.mergeToVert),
            runIndex: new Uint32Array(mesh.runIndex),
            runOriginalID: new Uint32Array(mesh.runOriginalID),
            runFlags: new Uint8Array(mesh.runFlags),
            faceID: new Uint32Array(mesh.faceID),
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
export async function loadManifoldKernelOps(): Promise<ManifoldKernelOps> {
  const module = await Module()
  const rawManifoldConstructor = module.Manifold
  module.setup()
  return createManifoldKernelOps(module, rawManifoldConstructor)
}
