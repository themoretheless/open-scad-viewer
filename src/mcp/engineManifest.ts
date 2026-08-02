import {
  canonicalJson,
  computeGeometryManifestDigest,
  GEOMETRY_MANIFEST_ARCHIVE,
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
    engine_class: engine.engineClass,
    display_name: engine.displayName,
    permanent: engine.permanent,
    maturity: engine.maturity,
    engine_key: engine.engineKey,
    kernel_fingerprint: engine.kernelFingerprint,
    semantic_program_version: engine.semanticProgramVersion,
    capability_manifest_version: engine.capabilityManifestVersion,
    manifest_digest: engine.manifestDigest,
    manifest_resource_uri: engineManifestUri(engine.engineClass, engine.capabilityManifestVersion),
    language_contracts: engine.languageContracts,
    input_contract: engine.inputContract,
    capabilities: engine.capabilities,
    planned_capabilities: engine.plannedCapabilities,
    qualities: engine.qualities,
    representations: engine.representations,
    planned_representations: engine.plannedRepresentations,
    export_formats: engine.exportFormats,
    planned_export_formats: engine.plannedExportFormats,
    limits: engine.limits,
    isolation: engine.isolation,
    deployment: engine.deployment,
    qualification: {
      status: engine.qualification.status,
      record_id: engine.qualification.recordId,
      corpus_version: engine.qualification.corpusVersion,
      target: engine.qualification.target,
    },
    dependency: {
      package_name: engine.dependency.packageName,
      version: engine.dependency.version,
      license_expression: engine.dependency.licenseExpression,
      sbom_ref: engine.dependency.sbomRef,
      sbom_sha256: engine.dependency.sbomSha256,
      lockfile_sha256: engine.dependency.lockfileSha256,
    },
    rollback_compatibility: {
      disable_engine_capability: engine.rollbackCompatibility.disableEngineCapability,
      source_contract_preserved: engine.rollbackCompatibility.sourceContractPreserved,
      cross_engine_fallback: engine.rollbackCompatibility.crossEngineFallback,
      minimum_catalog_schema: engine.rollbackCompatibility.minimumCatalogSchema,
    },
    automatic_fallback: engine.automaticFallback,
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
