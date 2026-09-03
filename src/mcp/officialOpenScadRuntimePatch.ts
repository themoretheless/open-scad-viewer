import { createHash } from 'node:crypto'

export const OFFICIAL_OPENSCAD_RUNTIME_VERSION = '2026.09.01' as const
export const OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL =
  'https://files.openscad.org/snapshots/OpenSCAD-2026.09.01-WebAssembly-node.zip' as const
export const OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256 =
  '82054dfb4911686de0ee3ea36771dbf81f3d014c3460c8ea069ab4f933f6d888' as const
export const OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION = 2 as const
export const OFFICIAL_OPENSCAD_RUNTIME_SHA256 =
  '80a6e6129ddf58e8262c8ff3023ee68afc93937b5a5d1afa54b054415c7b768c' as const
export const OFFICIAL_OPENSCAD_RUNTIME_FILENAME = 'openscad.patched.cjs' as const
export const OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME = 'runtime-manifest.json' as const
export const OFFICIAL_OPENSCAD_FONT_FAMILY = 'Basic' as const
export const OFFICIAL_OPENSCAD_FONT_FILENAME = 'Basic-Regular.ttf' as const
export const OFFICIAL_OPENSCAD_FONT_URL =
  'https://raw.githubusercontent.com/SorkinType/Basic/202e65ac93bd6977e83b2f10db6b1467e0b348db/Basic-Regular.ttf' as const
export const OFFICIAL_OPENSCAD_FONT_SHA256 =
  'f2487f20e2241002d007831ec0e7e9c24ad39e36b49492326c79f3d70fd3b270' as const
export const OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME = 'Basic-OFL.txt' as const
export const OFFICIAL_OPENSCAD_FONT_LICENSE_URL =
  'https://raw.githubusercontent.com/SorkinType/Basic/202e65ac93bd6977e83b2f10db6b1467e0b348db/OFL.txt' as const
export const OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256 =
  '25be5240815dc880cfad3606b03c9f05125252e9ce542f733cbbf87837ac810f' as const

export interface OfficialOpenScadRuntimeIntegrity {
  readonly runtimeSha256: string
  readonly fontSha256: string
  readonly fontLicenseSha256: string
}

export const OFFICIAL_OPENSCAD_PINNED_INTEGRITY: OfficialOpenScadRuntimeIntegrity = Object.freeze({
  runtimeSha256: OFFICIAL_OPENSCAD_RUNTIME_SHA256,
  fontSha256: OFFICIAL_OPENSCAD_FONT_SHA256,
  fontLicenseSha256: OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256,
})

const MODULE_BOOTSTRAP = 'var Module=typeof Module!="undefined"?Module:{};'
const MODULE_BOOTSTRAP_PATCH = [
  '/* open-scad-viewer-official-runtime-patch-v2 */',
  'var Module=globalThis.__OPENSCAD_MODULE__;',
  'if(!Module||typeof Module!=="object")',
  'throw new Error("E_OFFICIAL_OPENSCAD_MODULE_CONFIG");',
].join('')

const ENVIRONMENT_BOOTSTRAP = 'var ENV={};'
const ENVIRONMENT_BOOTSTRAP_PATCH = 'var ENV=Module["environment"]||{};'

const NODEFS_BOOTSTRAP = 'if(ENVIRONMENT_IS_NODE){NODEFS.staticInit()}'
const NODEFS_BOOTSTRAP_PATCH =
  'if(ENVIRONMENT_IS_NODE&&Module["useNodeRawFS"]===true){NODEFS.staticInit()}'

const NODERAWFS_INSTALL =
  'for(var _key in NODERAWFS){FS[_key]=_wrapNodeError(NODERAWFS[_key])}'
const NODERAWFS_INSTALL_PATCH =
  'if(Module["useNodeRawFS"]===true)for(var _key in NODERAWFS){FS[_key]=_wrapNodeError(NODERAWFS[_key])}'

export interface OfficialOpenScadRuntimeManifest {
  readonly schemaVersion: 2
  readonly runtimeVersion: typeof OFFICIAL_OPENSCAD_RUNTIME_VERSION
  readonly archiveUrl: typeof OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL
  readonly archiveSha256: typeof OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256
  readonly patchVersion: typeof OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION
  readonly runtimeFilename: typeof OFFICIAL_OPENSCAD_RUNTIME_FILENAME
  readonly runtimeSha256: string
  readonly fontFamily: typeof OFFICIAL_OPENSCAD_FONT_FAMILY
  readonly fontFilename: typeof OFFICIAL_OPENSCAD_FONT_FILENAME
  readonly fontUrl: typeof OFFICIAL_OPENSCAD_FONT_URL
  readonly fontSha256: string
  readonly fontLicenseFilename: typeof OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME
  readonly fontLicenseUrl: typeof OFFICIAL_OPENSCAD_FONT_LICENSE_URL
  readonly fontLicenseSha256: string
}

export class OfficialOpenScadRuntimePatchError extends Error {
  readonly code = 'E_OFFICIAL_OPENSCAD_PATCH' as const

  constructor(message: string) {
    super(message)
    this.name = 'OfficialOpenScadRuntimePatchError'
  }
}

function replaceExactlyOnce(source: string, needle: string, replacement: string): string {
  const first = source.indexOf(needle)
  const last = source.lastIndexOf(needle)
  if (first < 0 || first !== last) {
    throw new OfficialOpenScadRuntimePatchError(
      `Pinned OpenSCAD runtime patch sentinel occurred ${first < 0 ? 0 : 'more than 1'} times`,
    )
  }
  return source.slice(0, first) + replacement + source.slice(first + needle.length)
}

/**
 * Deterministically converts the pinned official Node build from NODERAWFS to
 * an injected Module backed by Emscripten MEMFS. Node's permission model is a
 * second boundary in the child process; this patch removes the runtime's need
 * for process.binding() and makes accidental host-path access fail closed.
 */
export function patchOfficialOpenScadRuntimeSource(source: string): string {
  if (typeof source !== 'string' || source.length < 1_000_000) {
    throw new OfficialOpenScadRuntimePatchError('Pinned OpenSCAD runtime source is missing or truncated')
  }
  if (/open-scad-viewer-official-runtime-patch-v\d+/u.test(source)) {
    throw new OfficialOpenScadRuntimePatchError('Pinned OpenSCAD runtime source is already patched')
  }

  let patched = replaceExactlyOnce(source, MODULE_BOOTSTRAP, MODULE_BOOTSTRAP_PATCH)
  patched = replaceExactlyOnce(patched, ENVIRONMENT_BOOTSTRAP, ENVIRONMENT_BOOTSTRAP_PATCH)
  patched = replaceExactlyOnce(patched, NODEFS_BOOTSTRAP, NODEFS_BOOTSTRAP_PATCH)
  patched = replaceExactlyOnce(patched, NODERAWFS_INSTALL, NODERAWFS_INSTALL_PATCH)
  return patched
}

export function sha256Buffer(value: Uint8Array | string): string {
  return createHash('sha256').update(value).digest('hex')
}

export function createOfficialOpenScadRuntimeManifest(
  patchedRuntime: Uint8Array | string,
  font: Uint8Array | string = OFFICIAL_OPENSCAD_FONT_SHA256,
  fontLicense: Uint8Array | string = OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256,
): OfficialOpenScadRuntimeManifest {
  return Object.freeze({
    schemaVersion: 2,
    runtimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
    archiveUrl: OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL,
    archiveSha256: OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
    patchVersion: OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
    runtimeFilename: OFFICIAL_OPENSCAD_RUNTIME_FILENAME,
    runtimeSha256: sha256Buffer(patchedRuntime),
    fontFamily: OFFICIAL_OPENSCAD_FONT_FAMILY,
    fontFilename: OFFICIAL_OPENSCAD_FONT_FILENAME,
    fontUrl: OFFICIAL_OPENSCAD_FONT_URL,
    fontSha256: typeof font === 'string' && font === OFFICIAL_OPENSCAD_FONT_SHA256
      ? font
      : sha256Buffer(font),
    fontLicenseFilename: OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
    fontLicenseUrl: OFFICIAL_OPENSCAD_FONT_LICENSE_URL,
    fontLicenseSha256: typeof fontLicense === 'string'
      && fontLicense === OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256
      ? fontLicense
      : sha256Buffer(fontLicense),
  })
}

export function isOfficialOpenScadRuntimeManifest(
  value: unknown,
): value is OfficialOpenScadRuntimeManifest {
  if (value === null || Array.isArray(value) || typeof value !== 'object') return false
  const manifest = value as Record<string, unknown>
  const keys = Object.keys(manifest)
  return keys.length === 14
    && keys.every(key => [
      'schemaVersion', 'runtimeVersion', 'archiveUrl', 'archiveSha256',
      'patchVersion', 'runtimeFilename', 'runtimeSha256', 'fontFamily',
      'fontFilename', 'fontUrl', 'fontSha256', 'fontLicenseFilename',
      'fontLicenseUrl', 'fontLicenseSha256',
    ].includes(key))
    && manifest.schemaVersion === 2
    && manifest.runtimeVersion === OFFICIAL_OPENSCAD_RUNTIME_VERSION
    && manifest.archiveUrl === OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL
    && manifest.archiveSha256 === OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256
    && manifest.patchVersion === OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION
    && manifest.runtimeFilename === OFFICIAL_OPENSCAD_RUNTIME_FILENAME
    && typeof manifest.runtimeSha256 === 'string'
    && /^[0-9a-f]{64}$/.test(manifest.runtimeSha256)
    && manifest.fontFamily === OFFICIAL_OPENSCAD_FONT_FAMILY
    && manifest.fontFilename === OFFICIAL_OPENSCAD_FONT_FILENAME
    && manifest.fontUrl === OFFICIAL_OPENSCAD_FONT_URL
    && typeof manifest.fontSha256 === 'string'
    && /^[0-9a-f]{64}$/.test(manifest.fontSha256)
    && manifest.fontLicenseFilename === OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME
    && manifest.fontLicenseUrl === OFFICIAL_OPENSCAD_FONT_LICENSE_URL
    && typeof manifest.fontLicenseSha256 === 'string'
    && /^[0-9a-f]{64}$/.test(manifest.fontLicenseSha256)
}

export function matchesOfficialOpenScadRuntimeIntegrity(
  manifest: OfficialOpenScadRuntimeManifest,
  expected: OfficialOpenScadRuntimeIntegrity = OFFICIAL_OPENSCAD_PINNED_INTEGRITY,
): boolean {
  return manifest.runtimeSha256 === expected.runtimeSha256
    && manifest.fontSha256 === expected.fontSha256
    && manifest.fontLicenseSha256 === expected.fontLicenseSha256
}
