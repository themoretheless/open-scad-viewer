import { createHash } from 'node:crypto'
import { readFile, realpath } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

export const QUALIFICATION_PLAYWRIGHT_PACKAGE = Object.freeze({
  name: '@open-scad-viewer/browser-qualification',
  version: '0.0.0',
  packageManager: 'npm@10.9.8',
  nodeEngine: '>=20.19.0',
  playwrightVersion: '1.62.1',
  playwrightDependency: 'playwright@1.62.1',
  relativeRoot: 'tools/browser-qualification',
  licenseManifestPath: 'docs/qualification/playwright-license-manifest-v1.json',
  licenseManifestSha256: 'ca6b12d581fdb7added7c5b7e144a8b5b20ae946417ec6e663a8bda348df1b6a',
  installCommand: 'npm ci --prefix tools/browser-qualification --ignore-scripts --include=dev --registry=https://registry.npmjs.org/ --audit=false --fund=false',
  browserProvisionEnvironment: 'ubuntu-linux-posix',
  packageJsonSha256: '639d9e0c47a413b3d87e86fc0ef6e6aae546034810d475c5f58ff9ce8835b0e4',
  packageLockSha256: '0a2e7f09703398ff090367d55e1f39b88b363e5267126edf699a2e78fedc471f',
})

const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
const packageRoot = join(repositoryRoot, QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot)
const packageJsonPath = join(packageRoot, 'package.json')
const packageLockPath = join(packageRoot, 'package-lock.json')
const licenseManifestPath = join(repositoryRoot, QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestPath)
const isolatedNodeModulesRoot = join(packageRoot, 'node_modules')
const qualificationRequire = createRequire(packageJsonPath)
const admittedRuntimePackages = new Set(['playwright', 'playwright-core', 'fsevents'])

export class QualificationPlaywrightPackageError extends Error {
  constructor(code, message, details = {}, options = undefined) {
    super(message, options)
    this.name = 'QualificationPlaywrightPackageError'
    this.code = code
    this.details = Object.freeze({ ...details })
  }
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex')
}

function inside(parent, candidate) {
  const path = relative(resolve(parent), resolve(candidate))
  return path.length > 0 && path !== '..' && !path.startsWith('..' + sep) && !isAbsolute(path)
}

function exactPlaywrightDependency(candidate) {
  return candidate !== null
    && typeof candidate === 'object'
    && !Array.isArray(candidate)
    && Object.keys(candidate).length === 1
    && candidate.playwright === QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion
}

function integrityError(message, details = {}) {
  return new QualificationPlaywrightPackageError(
    'E_QUALIFICATION_PLAYWRIGHT_PACKAGE_INTEGRITY',
    message,
    details,
  )
}

export async function inspectQualificationPlaywrightPackage() {
  let packageBytes
  let lockBytes
  let licenseManifestBytes
  try {
    ;[packageBytes, lockBytes, licenseManifestBytes] = await Promise.all([
      readFile(packageJsonPath),
      readFile(packageLockPath),
      readFile(licenseManifestPath),
    ])
  } catch (cause) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_PACKAGE_MISSING',
      'The isolated browser qualification package or lockfile is unavailable',
      { packageRoot: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot },
      { cause },
    )
  }
  const packageJsonSha256 = sha256(packageBytes)
  const packageLockSha256 = sha256(lockBytes)
  const licenseManifestSha256 = sha256(licenseManifestBytes)
  if (packageJsonSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.packageJsonSha256
      || packageLockSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.packageLockSha256
      || licenseManifestSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256) {
    throw integrityError('The isolated browser qualification package bytes changed', {
      expectedPackageJsonSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageJsonSha256,
      actualPackageJsonSha256: packageJsonSha256,
      expectedPackageLockSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageLockSha256,
      actualPackageLockSha256: packageLockSha256,
      expectedLicenseManifestSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256,
      actualLicenseManifestSha256: licenseManifestSha256,
    })
  }
  let manifest
  let lock
  let licenseManifest
  try {
    manifest = JSON.parse(packageBytes.toString('utf8'))
    lock = JSON.parse(lockBytes.toString('utf8'))
    licenseManifest = JSON.parse(licenseManifestBytes.toString('utf8'))
  } catch (cause) {
    throw integrityError('The isolated browser qualification package is not valid JSON', {
      cause: cause instanceof Error ? cause.message : String(cause),
    })
  }
  if (manifest.name !== QUALIFICATION_PLAYWRIGHT_PACKAGE.name
      || manifest.version !== QUALIFICATION_PLAYWRIGHT_PACKAGE.version
      || manifest.private !== true
      || manifest.packageManager !== QUALIFICATION_PLAYWRIGHT_PACKAGE.packageManager
      || manifest.engines?.node !== QUALIFICATION_PLAYWRIGHT_PACKAGE.nodeEngine
      || !exactPlaywrightDependency(manifest.devDependencies)
      || manifest.dependencies !== undefined) {
    throw integrityError('The isolated browser qualification manifest contract changed')
  }
  const lockRoot = lock.packages?.['']
  const lockedPlaywright = lock.packages?.['node_modules/playwright']
  const lockedPlaywrightCore = lock.packages?.['node_modules/playwright-core']
  if (lock.name !== QUALIFICATION_PLAYWRIGHT_PACKAGE.name
      || lock.version !== QUALIFICATION_PLAYWRIGHT_PACKAGE.version
      || lock.lockfileVersion !== 3
      || lockRoot?.name !== QUALIFICATION_PLAYWRIGHT_PACKAGE.name
      || lockRoot?.version !== QUALIFICATION_PLAYWRIGHT_PACKAGE.version
      || lockRoot?.engines?.node !== QUALIFICATION_PLAYWRIGHT_PACKAGE.nodeEngine
      || !exactPlaywrightDependency(lockRoot?.devDependencies)
      || lockedPlaywright?.version !== QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion
      || lockedPlaywright?.dependencies?.['playwright-core']
        !== QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion
      || lockedPlaywrightCore?.version !== QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion) {
    throw integrityError('The isolated browser qualification lock graph changed')
  }
  if (licenseManifest?.schema !== 'open-scad-viewer/playwright-license-manifest'
      || licenseManifest.version !== 1
      || licenseManifest.qualificationClaim !== 'none'
      || licenseManifest.sourceLock?.sha256 !== packageLockSha256
      || !Array.isArray(licenseManifest.packages)
      || licenseManifest.packages.length !== 3) {
    throw integrityError('The Playwright qualification license manifest contract changed')
  }
  const lockedPackages = new Map([
    ['playwright', lockedPlaywright],
    ['playwright-core', lockedPlaywrightCore],
    ['fsevents', lock.packages?.['node_modules/fsevents']],
  ])
  for (const packageEvidence of licenseManifest.packages) {
    const locked = lockedPackages.get(packageEvidence?.name)
    if (locked === undefined
        || packageEvidence.version !== locked.version
        || packageEvidence.license !== locked.license
        || packageEvidence.integrity !== locked.integrity
        || !Array.isArray(packageEvidence.licenseFiles)
        || packageEvidence.licenseFiles.length === 0) {
      throw integrityError('The Playwright qualification license package binding changed', {
        packageName: packageEvidence?.name ?? null,
      })
    }
    for (const file of packageEvidence.licenseFiles) {
      if (typeof file?.path !== 'string' || file.path.length === 0
          || file.path.startsWith('/') || file.path.split('/').includes('..')
          || !Number.isSafeInteger(file.byteLength) || file.byteLength < 1
          || !/^[0-9a-f]{64}$/.test(file.sha256 ?? '')) {
        throw integrityError('The Playwright qualification license file binding changed', {
          packageName: packageEvidence.name,
        })
      }
    }
  }
  return Object.freeze({
    packageRoot: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot,
    packageJsonPath: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot + '/package.json',
    packageLockPath: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot + '/package-lock.json',
    packageJsonSha256,
    packageLockSha256,
    licenseManifestPath: QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestPath,
    licenseManifestSha256,
    playwrightVersion: QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion,
  })
}

export async function verifyQualificationPlaywrightLicenseFiles() {
  let manifest
  try {
    const bytes = await readFile(licenseManifestPath)
    if (sha256(bytes) !== QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256) {
      throw integrityError('The Playwright qualification license manifest bytes changed')
    }
    manifest = JSON.parse(bytes.toString('utf8'))
  } catch (cause) {
    if (cause instanceof QualificationPlaywrightPackageError) throw cause
    throw integrityError('The Playwright qualification license manifest is unavailable', {
      cause: cause instanceof Error ? cause.message : String(cause),
    })
  }
  const verified = []
  for (const packageEvidence of manifest.packages) {
    const packageDirectory = join(isolatedNodeModulesRoot, packageEvidence.name)
    let present = true
    try {
      await realpath(packageDirectory)
    } catch (cause) {
      if (packageEvidence.optionalFor !== process.platform) {
        present = false
      } else {
        throw integrityError('A platform-required qualification license package is absent', {
          packageName: packageEvidence.name,
          platform: process.platform,
          cause: cause instanceof Error ? cause.message : String(cause),
        })
      }
    }
    if (!present) continue
    for (const file of packageEvidence.licenseFiles) {
      const lexicalPath = join(packageDirectory, ...file.path.split('/'))
      const actualPath = await assertIsolatedPackageResolution(packageEvidence.name, lexicalPath)
      const bytes = await readFile(actualPath)
      if (bytes.byteLength !== file.byteLength || sha256(bytes) !== file.sha256) {
        throw integrityError('An installed Playwright qualification license file changed', {
          packageName: packageEvidence.name,
          path: file.path,
        })
      }
      verified.push(Object.freeze({
        packageName: packageEvidence.name,
        path: file.path,
        byteLength: bytes.byteLength,
        sha256: file.sha256,
      }))
    }
  }
  return Object.freeze(verified)
}

export async function assertIsolatedPackageResolution(
  packageName,
  resolvedPath,
  options = {},
) {
  if (!admittedRuntimePackages.has(packageName)) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_RESOLUTION_ESCAPE',
      'An unrecognized package was requested from the isolated Playwright graph',
      { packageName },
    )
  }
  const isolatedPackageRoot = join(isolatedNodeModulesRoot, packageName)
  if (typeof resolvedPath !== 'string' || !isAbsolute(resolvedPath)
      || !inside(isolatedPackageRoot, resolvedPath)) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_RESOLUTION_ESCAPE',
      `${packageName} resolved outside its isolated qualification-only package`,
      {
        packageName,
        resolvedPath: typeof resolvedPath === 'string' ? resolvedPath : null,
        requiredRoot: `${QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot}/node_modules/${packageName}`,
      },
    )
  }
  const canonicalize = options.realpath ?? realpath
  let actualPackageRoot
  let actualNodeModulesRoot
  let actualDependencyRoot
  let actualPath
  try {
    ;[actualPackageRoot, actualNodeModulesRoot, actualDependencyRoot, actualPath] = await Promise.all([
      canonicalize(packageRoot),
      canonicalize(isolatedNodeModulesRoot),
      canonicalize(isolatedPackageRoot),
      canonicalize(resolvedPath),
    ])
  } catch (cause) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_UNAVAILABLE',
      `The isolated ${packageName} installation is unavailable`,
      { packageName, packageRoot: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot },
      { cause },
    )
  }
  const expectedNodeModulesRoot = join(actualPackageRoot, 'node_modules')
  const expectedDependencyRoot = join(expectedNodeModulesRoot, packageName)
  if (resolve(actualNodeModulesRoot) !== resolve(expectedNodeModulesRoot)
      || resolve(actualDependencyRoot) !== resolve(expectedDependencyRoot)
      || (actualPath !== actualDependencyRoot && !inside(actualDependencyRoot, actualPath))) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_RESOLUTION_ESCAPE',
      `The isolated ${packageName} installation escapes through a symbolic link`,
      {
        packageName,
        requiredRoot: `${QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot}/node_modules/${packageName}`,
      },
    )
  }
  return actualPath
}

export function assertIsolatedPlaywrightResolution(resolvedPath, options = {}) {
  return assertIsolatedPackageResolution('playwright', resolvedPath, options)
}

export async function loadQualificationPlaywrightPackage(options = {}) {
  const packageMetadata = await inspectQualificationPlaywrightPackage()
  const resolveModule = options.resolveModule ?? ((specifier) => qualificationRequire.resolve(specifier))
  const loadModule = options.loadModule ?? ((path) => qualificationRequire(path))
  let modulePath
  let manifestPath
  let coreModulePath
  let coreManifestPath
  try {
    modulePath = resolveModule('playwright')
    manifestPath = resolveModule('playwright/package.json')
    coreModulePath = resolveModule('playwright-core')
    coreManifestPath = resolveModule('playwright-core/package.json')
  } catch (cause) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_UNAVAILABLE',
      'Playwright is not installed in the isolated qualification-only package',
      {
        packageRoot: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot,
        installCommand: QUALIFICATION_PLAYWRIGHT_PACKAGE.installCommand,
      },
      { cause },
    )
  }
  ;[modulePath, manifestPath, coreModulePath, coreManifestPath] = await Promise.all([
    assertIsolatedPlaywrightResolution(modulePath),
    assertIsolatedPlaywrightResolution(manifestPath),
    assertIsolatedPackageResolution('playwright-core', coreModulePath),
    assertIsolatedPackageResolution('playwright-core', coreManifestPath),
  ])
  let imported
  let installedManifest
  let installedCoreManifest
  try {
    imported = await loadModule(modulePath)
    ;[installedManifest, installedCoreManifest] = await Promise.all([
      readFile(manifestPath, 'utf8').then(JSON.parse),
      readFile(coreManifestPath, 'utf8').then(JSON.parse),
    ])
  } catch (cause) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_UNAVAILABLE',
      'The isolated Playwright module or installed manifest could not be loaded',
      { packageRoot: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot },
      { cause },
    )
  }
  const playwright = imported?.default ?? imported
  if (playwright === null || typeof playwright !== 'object'
      || installedManifest.name !== 'playwright'
      || installedManifest.version !== QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion
      || installedCoreManifest.name !== 'playwright-core'
      || installedCoreManifest.version !== QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion) {
    throw new QualificationPlaywrightPackageError(
      'E_QUALIFICATION_PLAYWRIGHT_VERSION',
      'The isolated Playwright module does not match its exact qualification version',
      {
        expectedVersion: QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion,
        observedName: installedManifest?.name ?? null,
        observedVersion: installedManifest?.version ?? null,
        observedCoreName: installedCoreManifest?.name ?? null,
        observedCoreVersion: installedCoreManifest?.version ?? null,
      },
    )
  }
  const licenseFiles = await verifyQualificationPlaywrightLicenseFiles()
  return Object.freeze({
    playwright,
    manifest: installedManifest,
    coreManifest: installedCoreManifest,
    licenseFiles,
    packageMetadata,
  })
}

export function isolatedBrowserProvisionCommand(browser) {
  return 'tools/browser-qualification/node_modules/.bin/playwright install ' + browser
}
