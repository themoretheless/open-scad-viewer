import {
  canonicalJson,
  computeGeometryManifestDigest,
  GEOMETRY_MANIFEST_ARCHIVE,
  toWireManifest,
  type GeometryEngineClass,
  type GeometryEngineStaticManifest,
} from '../core/geometryExecution'

export { canonicalJson, computeGeometryManifestDigest }

export function engineManifestUri(
  engineClass: GeometryEngineClass,
  manifestVersion: string,
): string {
  return `openscad://engines/${encodeURIComponent(engineClass)}/capabilities/${encodeURIComponent(manifestVersion)}`
}

export function staticEngineManifestToWire(engine: GeometryEngineStaticManifest) {
  return {
    ...toWireManifest(engine),
    manifest_digest: engine.manifestDigest,
    manifest_resource_uri: engineManifestUri(engine.engineClass, engine.capabilityManifestVersion),
  }
}

export function assertGeometryManifestDigest(engine: GeometryEngineStaticManifest): void {
  const actualDigest = computeGeometryManifestDigest(engine)
  if (actualDigest !== engine.manifestDigest) {
    throw new Error(`Immutable geometry manifest ${engine.capabilityManifestVersion} digest mismatch`)
  }
}

export function archivedGeometryManifest(
  manifestVersion: string,
): GeometryEngineStaticManifest | null {
  const archived = GEOMETRY_MANIFEST_ARCHIVE[
    manifestVersion as keyof typeof GEOMETRY_MANIFEST_ARCHIVE
  ]
  if (!archived) return null
  assertGeometryManifestDigest(archived)
  return archived
}

export function immutableEngineManifestToWire(
  engineClass: GeometryEngineClass,
  manifestVersion: string,
) {
  const archived = archivedGeometryManifest(manifestVersion)
  if (!archived || archived.engineClass !== engineClass) return null
  return staticEngineManifestToWire(archived)
}

export function assertGeometryManifestArchive(): void {
  Object.values(GEOMETRY_MANIFEST_ARCHIVE).forEach(assertGeometryManifestDigest)
}
