import { LEGACY_FORMAT_SET, OPENSCAD_2021_LEGACY_IMPORT_FORMATS, OpenScadImportDataError, OpenScadImportError } from '../openScadImport'
import type { OpenScad2021LegacyImportFormat, OpenScadDxfDimensionMetadata, OpenScadDxfQueryDiagnostic, OpenScadDxfQueryDiagnosticCode, OpenScadDxfQueryMetadata, OpenScadDxfQueryOptions, OpenScadDxfQueryResult, OpenScadImportFormat, OpenScadImportGeometry, OpenScadImportLoadOptions, OpenScadImportPoint2, OpenScadImportPreparationBudget, OpenScadImportPreparationOptions, PreparedOpenScadImportAssets, ResolvedDxfQueryMetadata, ResolvedDxfQueryTransform } from '../openScadImport'
import { OPENSCAD_PROJECT_MAX_FILES, OpenScadProject, OpenScadProjectError, normalizeOpenScadProjectPath, resolveOpenScadProjectPath } from '../openScadProject'
import type { OpenScadProjectFile } from '../openScadProject'
import { cloneGeometry, dataError, formatOfPath, geometry3D } from './parseOpenScadStl'
import { forcedImportAssetKey, prepareOpenScadImportAsset, svgProjectOptions } from './parseOpenScadDxfQueryMetadata'
import { geometry2D, parseOpenScadSvg } from './parseOpenScad3mf'

/** Decode every supported project asset once, without consulting the host filesystem. */
export async function prepareOpenScadImportAssets(
  project: OpenScadProject,
  options: OpenScadImportPreparationOptions = {},
): Promise<PreparedOpenScadImportAssets> {
  const assets = new Map<string, OpenScadImportGeometry | OpenScadImportDataError>()
  const forcedAssets = new Map<string, OpenScadImportGeometry | OpenScadImportDataError>()
  const dxfQueryMetadata = new Map<string, OpenScadDxfQueryMetadata | OpenScadImportDataError>()
  const svgOptions = project.list().some(file => /\.svg$/i.test(file.path)) ? svgProjectOptions(project) : {}
  const budget: OpenScadImportPreparationBudget = {
    vertices: 0,
    triangles: 0,
    points: 0,
    zipExpandedBytes: 0,
    dxfQueryRecords: 0,
  }
  for (const summary of project.list()) {
    const format = formatOfPath(summary.path)
    if (format === null) continue
    const file = project.read(summary.path)!
    const prepared = await prepareOpenScadImportAsset(file, format, budget, svgOptions)
    assets.set(summary.path, prepared.asset)
    if (prepared.dxfQueryMetadata !== undefined) {
      dxfQueryMetadata.set(summary.path, prepared.dxfQueryMetadata)
    }
  }

  const requests = options.forcedAssets ?? []
  if (requests.length > OPENSCAD_PROJECT_MAX_FILES * OPENSCAD_2021_LEGACY_IMPORT_FORMATS.length) {
    return dataError(
      'E_IMPORT_LIMIT',
      'Forced legacy import preparation request limit exceeded.',
      undefined,
      OPENSCAD_PROJECT_MAX_FILES * OPENSCAD_2021_LEGACY_IMPORT_FORMATS.length,
      requests.length,
    )
  }
  for (const request of requests) {
    if (!LEGACY_FORMAT_SET.has(request.format)) {
      throw new TypeError(`Unsupported forced legacy import format ${String(request.format)}`)
    }
    const path = normalizeOpenScadProjectPath(request.path)
    const key = forcedImportAssetKey(path, request.format)
    if (forcedAssets.has(key)) continue
    const inferredFormat = formatOfPath(path)
    if (inferredFormat === request.format && assets.has(path)) {
      forcedAssets.set(key, assets.get(path)!)
      continue
    }
    const file = project.read(path)
    if (file === null) continue
    const prepared = await prepareOpenScadImportAsset(file, request.format, budget)
    forcedAssets.set(key, prepared.asset)
    if (prepared.dxfQueryMetadata !== undefined && !dxfQueryMetadata.has(path)) {
      dxfQueryMetadata.set(path, prepared.dxfQueryMetadata)
    }
  }
  return Object.freeze({ assets, forcedAssets, dxfQueryMetadata })
}

export function contextualImportError(
  error: OpenScadImportDataError,
  sourcePath: string,
  specifier: string,
  assetPath: string,
  format: OpenScadImportFormat,
): OpenScadImportError {
  return new OpenScadImportError(error.code, error.message, {
    sourcePath,
    specifier,
    assetPath,
    format,
    ...(error.limit === undefined ? {} : { limit: error.limit }),
    ...(error.actual === undefined ? {} : { actual: error.actual }),
  })
}

export function validateImportOptions(
  options: OpenScadImportLoadOptions,
  sourcePath: string,
  specifier: string,
  assetPath: string,
): Required<Pick<OpenScadImportLoadOptions, 'origin' | 'scale' | 'center' | 'dpi'>> & Pick<OpenScadImportLoadOptions, 'layer'> {
  const origin = options.origin ?? [0, 0]
  const scale = options.scale ?? 1
  const dpi = options.dpi ?? 72
  if (!Array.isArray(origin) || origin.length !== 2 || !origin.every(Number.isFinite)) {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() origin must contain two finite numbers.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (typeof scale !== 'number' || !Number.isFinite(scale) || !(scale > 0)) {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() scale must be a positive finite number.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (typeof dpi !== 'number' || !Number.isFinite(dpi) || !(dpi > 0)) {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() dpi must be a positive finite number.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (options.layer !== undefined && typeof options.layer !== 'string') {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() layer must be a string.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (options.center !== undefined && typeof options.center !== 'boolean') {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() center must be boolean.', {
      sourcePath, specifier, assetPath,
    })
  }
  return { origin: [origin[0], origin[1]], scale, center: options.center ?? false, dpi, layer: options.layer }
}

export function transformLoadedGeometry(
  source: OpenScadImportGeometry,
  options: ReturnType<typeof validateImportOptions>,
): OpenScadImportGeometry {
  if (source.dimension === 2) {
    const selected = source.format === 'dxf' && options.layer
      ? source.regions.filter(region => region.layer === options.layer)
      : source.regions
    if (selected.length === 0) return dataError('E_IMPORT_EMPTY', `${source.format.toUpperCase()} layer contains no filled geometry.`, source.format)
    const applyDxfOptions = source.format === 'dxf'
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity
    const regions = selected.map(region => ({
      ...region,
      contours: region.contours.map(contour => contour.map(point => {
        const x = applyDxfOptions ? (point[0] - options.origin[0]) * options.scale : point[0]
        const y = applyDxfOptions ? (point[1] - options.origin[1]) * options.scale : point[1]
        minX = Math.min(minX, x); maxX = Math.max(maxX, x)
        minY = Math.min(minY, y); maxY = Math.max(maxY, y)
        return [x, y] as const
      })),
    }))
    const centerX = options.center ? (minX + maxX) / 2 : 0
    const centerY = options.center ? (minY + maxY) / 2 : 0
    return geometry2D(source.format, regions.map(region => ({
      ...region,
      contours: region.contours.map(contour => contour.map(point => [point[0] - centerX, point[1] - centerY] as const)),
    })))
  }
  const vertices = [...source.vertices]
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity, minZ = Infinity, maxZ = -Infinity
  for (let offset = 0; offset < vertices.length; offset += 3) {
    minX = Math.min(minX, vertices[offset]); maxX = Math.max(maxX, vertices[offset])
    minY = Math.min(minY, vertices[offset + 1]); maxY = Math.max(maxY, vertices[offset + 1])
    minZ = Math.min(minZ, vertices[offset + 2]); maxZ = Math.max(maxZ, vertices[offset + 2])
  }
  if (options.center) {
    const centerX = (minX + maxX) / 2, centerY = (minY + maxY) / 2, centerZ = (minZ + maxZ) / 2
    for (let offset = 0; offset < vertices.length; offset += 3) {
      vertices[offset] -= centerX
      vertices[offset + 1] -= centerY
      vertices[offset + 2] -= centerZ
    }
  }
  return geometry3D(source.format, vertices, [...source.triangles])
}

export function resolveImportAsset(
  project: OpenScadProject,
  sourcePath: string,
  specifier: string,
  forcedFormat?: OpenScad2021LegacyImportFormat,
): { readonly assetPath: string; readonly file: OpenScadProjectFile; readonly format: OpenScadImportFormat } {
  let assetPath: string
  try {
    assetPath = resolveOpenScadProjectPath(sourcePath, specifier)
  } catch (error) {
    if (!(error instanceof OpenScadProjectError)) throw error
    throw new OpenScadImportError('E_IMPORT_PATH_INVALID', error.message, { sourcePath, specifier })
  }
  const extension = assetPath.slice(assetPath.lastIndexOf('.') + 1).toLowerCase()
  if (forcedFormat === undefined && extension === 'nef3') {
    throw new OpenScadImportError(
      'E_IMPORT_NEF3_UNAVAILABLE',
      'NEF3 import is only available in CGAL-enabled OpenSCAD builds; this independent runtime uses Manifold.',
      { sourcePath, specifier, assetPath },
    )
  }
  if (forcedFormat !== undefined && !LEGACY_FORMAT_SET.has(forcedFormat)) {
    throw new OpenScadImportError(
      'E_IMPORT_FORMAT_UNSUPPORTED',
      `Unsupported forced legacy import format ${String(forcedFormat)}.`,
      { sourcePath, specifier, assetPath },
    )
  }
  const format = forcedFormat ?? formatOfPath(assetPath)
  if (format === null) {
    throw new OpenScadImportError(
      'E_IMPORT_FORMAT_UNSUPPORTED',
      `import() does not support the extension of ${assetPath}.`,
      { sourcePath, specifier, assetPath },
    )
  }
  const file = project.read(assetPath)
  if (file === null) {
    throw new OpenScadImportError('E_IMPORT_FILE_MISSING', `Imported project asset ${assetPath} is missing.`, {
      sourcePath, specifier, assetPath, format,
    })
  }
  return { assetPath, file, format }
}

/** Resolve and clone a pre-decoded import using only module-relative project paths. */
export function loadPreparedOpenScadImport(
  project: OpenScadProject,
  prepared: PreparedOpenScadImportAssets,
  sourcePath: string,
  specifier: string,
  options: OpenScadImportLoadOptions = {},
): OpenScadImportGeometry {
  const { assetPath, file, format } = resolveImportAsset(
    project,
    sourcePath,
    specifier,
    options.forcedFormat,
  )
  const validated = validateImportOptions(options, sourcePath, specifier, assetPath)
  let decoded = options.forcedFormat === undefined
    ? prepared.assets.get(assetPath)
    : prepared.forcedAssets.get(forcedImportAssetKey(assetPath, options.forcedFormat))
      ?? (formatOfPath(assetPath) === options.forcedFormat ? prepared.assets.get(assetPath) : undefined)
  if (format === 'svg' && validated.dpi !== 72) {
    try {
      decoded = parseOpenScadSvg(file, validated.dpi, svgProjectOptions(project))
    } catch (error) {
      if (!(error instanceof OpenScadImportDataError)) throw error
      throw contextualImportError(error, sourcePath, specifier, assetPath, format)
    }
  }
  if (decoded === undefined) {
    throw new OpenScadImportError('E_IMPORT_INVALID_DATA', `Imported project asset ${assetPath} was not prepared.`, {
      sourcePath, specifier, assetPath, format,
    })
  }
  if (decoded instanceof OpenScadImportDataError) {
    throw contextualImportError(decoded, sourcePath, specifier, assetPath, format)
  }
  try {
    return transformLoadedGeometry(cloneGeometry(decoded), validated)
  } catch (error) {
    if (!(error instanceof OpenScadImportDataError)) throw error
    throw contextualImportError(error, sourcePath, specifier, assetPath, format)
  }
}

export function dxfQueryDiagnostic(
  code: OpenScadDxfQueryDiagnosticCode,
  message: string,
  sourcePath: string,
  specifier: string,
  details: Omit<OpenScadDxfQueryDiagnostic, 'severity' | 'code' | 'message' | 'sourcePath' | 'specifier'> = {},
): OpenScadDxfQueryDiagnostic {
  return Object.freeze({ severity: 'warning', code, message, sourcePath, specifier, ...details })
}

export function dxfQueryResult<T>(
  value: T | undefined,
  diagnostics: readonly OpenScadDxfQueryDiagnostic[],
): OpenScadDxfQueryResult<T> {
  return Object.freeze({ value, diagnostics: Object.freeze([...diagnostics]) })
}

export function resolvePreparedDxfQueryMetadata(
  project: OpenScadProject,
  prepared: PreparedOpenScadImportAssets,
  sourcePath: string,
  specifier: string,
): ResolvedDxfQueryMetadata | OpenScadDxfQueryDiagnostic {
  let assetPath: string
  try {
    assetPath = resolveOpenScadProjectPath(sourcePath, specifier)
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error)
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE',
      `DXF query path could not be resolved: ${detail}`,
      sourcePath,
      specifier,
    )
  }
  if (project.read(assetPath) === null) {
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE',
      `DXF query asset ${assetPath} is missing from the project.`,
      sourcePath,
      specifier,
      { assetPath },
    )
  }
  const metadata = prepared.dxfQueryMetadata.get(assetPath)
  if (metadata === undefined) {
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_NOT_PREPARED',
      `DXF query asset ${assetPath} was not prepared as DXF.`,
      sourcePath,
      specifier,
      { assetPath },
    )
  }
  if (metadata instanceof OpenScadImportDataError) {
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_INVALID_DATA',
      `DXF query asset ${assetPath} could not be decoded: ${metadata.message}`,
      sourcePath,
      specifier,
      { assetPath, importCode: metadata.code },
    )
  }
  return Object.freeze({ metadata, assetPath })
}

export function resolveDxfQueryTransform(
  options: OpenScadDxfQueryOptions,
  sourcePath: string,
  specifier: string,
): ResolvedDxfQueryTransform {
  const diagnostics: OpenScadDxfQueryDiagnostic[] = []
  let origin: readonly [number, number] = [0, 0]
  if (options.origin !== undefined) {
    const supplied: unknown = options.origin
    if (Array.isArray(supplied) && supplied.length === 2
      && supplied.every(value => typeof value === 'number')) {
      origin = [supplied[0], supplied[1]]
      if (!origin.every(Number.isFinite)) {
        diagnostics.push(dxfQueryDiagnostic(
          'OPENSCAD_DXF_QUERY_INVALID_ORIGIN',
          'DXF query origin contains a non-finite coordinate; 2021.01 arithmetic is preserved.',
          sourcePath,
          specifier,
        ))
      }
    } else {
      diagnostics.push(dxfQueryDiagnostic(
        'OPENSCAD_DXF_QUERY_INVALID_ORIGIN',
        'DXF query origin was not a two-number vector; [0, 0] is used.',
        sourcePath,
        specifier,
      ))
    }
  }
  // The 2021.01 implementation silently keeps the default when getDouble()
  // cannot read the value, and deliberately does not reject NaN or infinity.
  const scale = typeof options.scale === 'number' ? options.scale : 1
  return Object.freeze({
    layer: typeof options.layer === 'string' ? options.layer : '',
    origin: Object.freeze([origin[0], origin[1]] as const),
    scale,
    diagnostics: Object.freeze(diagnostics),
  })
}

export function transformDxfDimensionCoordinates(
  dimension: OpenScadDxfDimensionMetadata,
  origin: readonly [number, number],
  scale: number,
): readonly OpenScadImportPoint2[] {
  if (!dimension.modelSpace) return dimension.coordinates
  return dimension.coordinates.map((point, index) => {
    // Groups 11/21, 12/22 and 16/26 are direction-like coordinates in the
    // upstream stream parser: they are scaled but do not receive origin.
    const subtractOrigin = index !== 1 && index !== 2 && index !== 6
    return [
      (point[0] - (subtractOrigin ? origin[0] : 0)) * scale,
      (point[1] - (subtractOrigin ? origin[1] : 0)) * scale,
    ] as const
  })
}
