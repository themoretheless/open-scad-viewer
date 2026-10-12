import { AbortedError, OpenSCADParseError } from '../openscadErrors'
import { CSS_COLORS, resetPalette } from '../openscadColors'
import type { RGBA } from '../openscadColors'
import { compileOpenSCAD } from '../openscadCompiler'
import type { CallNode, ExpressionArgument, ModuleNode, Statement, Value } from '../openscadCompiler'
import { CooperativeCheckpoint, MAX_SOURCE_LENGTH, MAX_TRIANGLES, enterStableStatementScope, resolveStableExpressionArguments } from '../openscadParser'
import type { EvalContext, ParseOptions, ParseResult, Shape, Shape3D } from '../openscadParser'
import type { MeshData, MeshProvenanceRun, MeshSourceReference } from '../../core/mesh'
import type { OpenScadProject } from '../openScadProject'
import { OpenScadStableScope } from '../openScadStableScope'
import { bindOpenScad, prepareOpenScadFrontEnd } from '../openscadBinder'
import { booleanShapes } from './transformStableChildren'
import { callExpressionArguments, evalNodes, evalTopLevel, stableCompatibilityForcedAssets, vectorValue } from './evalBuiltin'
import { clamp01, identity } from '../math3d'
import { createBrepRecordingKernelOps } from '../solid/brepRecorder'
import { createOpenScadStableRuntimeVariables } from '../openScadStableRuntime'
import { defaultGeometryKernel } from '../cadGeometryKernel'
import { evalExpression, isExclusiveView, warn } from './isExclusiveView'
import { evaluationError, isFunctionValue, isStableProfile, overlayDynamicVariables } from '../openScadStableExpressionEval'
import { formatOpenScadValue, isOpenScadRange } from '../openScadValueSemantics'
import { geometryAssetId } from '../../core/scene'
import { prepareOpenScadImportAssets } from '../openScadImport'
import { prepareOpenScadSurfaceAssets } from '../openScadSurface'
import { prepareOpenScadTextAssets } from '../openScadText'

export function identityValue(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(identityValue).join(',')}]`
  if (isFunctionValue(value)) return `function:${value.name ?? '<anonymous>'}`
  if (isOpenScadRange(value)) return `range:${formatOpenScadValue(value)}`
  if (value === undefined) return 'undef'
  if (typeof value === 'string') return JSON.stringify(value)
  return String(value)
}

export async function evalUserModule(
  call: CallNode,
  module: ModuleNode,
  ctx: EvalContext,
  definitionScope?: OpenScadStableScope,
): Promise<Shape[]> {
  let env = new Map(ctx.env)
  if (isStableProfile(ctx)) {
    if (definitionScope === undefined) {
      evaluationError(ctx, call.p, `Stable module ${module.name}() is missing its lexical scope`)
    }
    const moduleStack = [...ctx.moduleStack, module.name]
    const definitionEnv = new Map(definitionScope.env)
    overlayDynamicVariables(definitionEnv, ctx.env)
    definitionEnv.set('$children', call.children.length)
    definitionEnv.set('$parent_modules', moduleStack.length)
    const definitionContext: EvalContext = {
      ...ctx,
      env: definitionEnv,
      stableScope: definitionScope,
      scopeVisibleBefore: Number.POSITIVE_INFINITY,
      moduleStack,
    }
    const args = callExpressionArguments(call)
    const resolved = resolveStableExpressionArguments(
      args,
      module.params.map(parameter => parameter.name),
      ctx,
    )
    const callerValues = new Map<ExpressionArgument, Value>()
    for (const argument of args) callerValues.set(argument, evalExpression(argument.value, ctx))
    const parameterValues = new Map<string, Value>()
    for (const parameter of module.params) {
      const supplied = resolved.get(parameter.name)
      const value = supplied !== undefined
        ? callerValues.get(supplied)
        : parameter.defaultValue !== undefined
          ? evalExpression(parameter.defaultValue, { ...definitionContext, env: new Map(definitionEnv) })
          : undefined
      parameterValues.set(parameter.name, value)
    }
    env = new Map(definitionEnv)
    for (const [name, value] of parameterValues) env.set(name, value)
    return booleanShapes(await evalNodes(module.children, {
      ...definitionContext,
      env,
      callChildren: {
        statements: call.children,
        env: new Map(ctx.env),
        scope: ctx.stableScope ?? definitionScope,
        continuation: ctx.callChildren,
      },
    }, false), 'union', ctx, call.p, module.name)
  } else {
    const positional = Object.entries(call.args).filter(([name]) => name.startsWith('_')).sort(([a], [b]) => Number(a.slice(1)) - Number(b.slice(1)))
    for (let i = 0; i < module.params.length; i++) {
      const param = module.params[i]
      const expression = call.args[param.name] ?? positional[i]?.[1] ?? param.defaultValue
      env.set(param.name, expression ? evalExpression(expression, { ...ctx, env }) : undefined)
    }
  }
  return await evalNodes(module.children, {
    ...ctx,
    env,
    callChildren: call.children,
    moduleStack: [...ctx.moduleStack, module.name],
  }, false)
}

export function parseLegacyColor(value: Value, ctx: EvalContext, p: number): RGBA {
  if (Array.isArray(value)) {
    const channels = vectorValue(value, ctx, p, 'color')
    return [clamp01(channels[0] ?? 0.5), clamp01(channels[1] ?? 0.5), clamp01(channels[2] ?? 0.5), clamp01(channels[3] ?? 1)]
  }
  if (typeof value !== 'string') evaluationError(ctx, p, 'color() expects a name or RGB(A) vector')
  const named = CSS_COLORS[value.toLowerCase()]
  if (named) return [...named]
  const match = value.match(/^#([0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (match) {
    const hex = match[1]
    return [parseInt(hex.slice(0, 2), 16) / 255, parseInt(hex.slice(2, 4), 16) / 255, parseInt(hex.slice(4, 6), 16) / 255, hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1]
  }
  evaluationError(ctx, p, `Unknown color ${value}`)
}

/**
 * Warm the official geometry kernel, cached per JS realm. Exported so a
 * hosting worker can eagerly compile it at startup instead of paying the
 * download+compile cost on the first request. A rejected load is NOT cached:
 * one transient network failure must not brick every future parse, so the
 * cached promise is cleared on rejection and the next call retries.
 */
export function warmGeometryKernel(): Promise<void> {
  return defaultGeometryKernel.warm().then(() => undefined)
}

export async function parseInternal(
  source: string,
  options: ParseOptions,
  compileAst: () => readonly Statement[] = () => compileOpenSCAD(source, {
    languageProfile: options.languageProfile,
  }),
  project?: OpenScadProject,
): Promise<ParseResult> {
  const now = options.now ?? (() => performance.now())
  const startedAt = now()
  if (project === undefined && source.length > MAX_SOURCE_LENGTH) {
    throw new OpenSCADParseError(source, 0, `Source exceeds ${MAX_SOURCE_LENGTH.toLocaleString()} characters`)
  }
  resetPalette()
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  let ast: readonly Statement[]
  let bound
  let parseMs: number
  let bindMs: number
  if (project === undefined) {
    const prepared = prepareOpenScadFrontEnd(source, {
      languageProfile,
      compile: () => compileAst(),
      now,
    })
    ast = prepared.program
    bound = prepared.bound
    parseMs = prepared.compileMs
    bindMs = prepared.bindMs
  } else {
    ast = compileAst()
    const compiledAt = now()
    bound = bindOpenScad(ast, { languageProfile, source })
    const boundAt = now()
    parseMs = Math.max(0, compiledAt - startedAt)
    bindMs = Math.max(0, boundAt - compiledAt)
  }
  const forcedImportAssets = project === undefined
    ? []
    : stableCompatibilityForcedAssets(project, ast)
  const [importAssets, surfaceAssets, textAssets] = project === undefined
    ? [undefined, undefined, undefined]
    : await Promise.all([
        prepareOpenScadImportAssets(project, { forcedAssets: forcedImportAssets }),
        prepareOpenScadSurfaceAssets(project),
        prepareOpenScadTextAssets(project),
      ])
  const parsedAt = now()
  const kernelSession = await defaultGeometryKernel.openEvalSession()
  // Recording costs only a small JSON graph, but it is opt-in because the result
  // crosses a protocol that validates an exact set of keys.
  const exactRecorder = options.recordExactSolids === true
    ? createBrepRecordingKernelOps(kernelSession.kernel)
    : null
  const warnings: string[] = []
  const modules = new Map(bound.modules)
  const functions = new Map(bound.functions)
  const quality = options.quality ?? 'full'
  const env = languageProfile === 'openscad/stable-2021.01'
    ? new Map<string, Value>(Array.from(
        createOpenScadStableRuntimeVariables({ quality, animationTime: options.animationTime }),
        ([name, value]) => [name, Array.isArray(value) ? [...value] : value as Value],
      ))
    : new Map<string, Value>([['$fn', 0], ['$fa', 12], ['$fs', 2]])
  const sourceReferences = new Map<number, MeshSourceReference>()
  const reduced = { value: false }
  let ctx: EvalContext = {
    kernel: exactRecorder?.ops ?? kernelSession.kernel,
    source,
    project,
    importAssets,
    surfaceAssets,
    textAssets,
    languageProfile,
    env,
    functions,
    modules,
    warnings,
    quality,
    sourceReferences,
    depth: 0,
    functionStack: [],
    moduleStack: [],
    budget: { ops: 0 },
    reduced,
    instancePath: 'root',
    valueBudget: { used: 0 },
    valueWeights: new WeakMap(),
    valueDepths: new WeakMap(),
  }
  if (isStableProfile(ctx)) ctx = enterStableStatementScope(ast, ctx)
  const initializedAt = now()
  const control = new CooperativeCheckpoint(options.shouldAbort, options.onYield, now, options.yieldControl)
  ctx = { ...ctx, control }

  try {
    let shapes = await evalTopLevel(ast, ctx, control)
    if (isStableProfile(ctx)) shapes = booleanShapes(shapes, 'union', ctx, 0, 'union')
    // A single giant statement may occupy a whole macrotask. Always yield
    // before post-processing so a queued cancel can skip mesh extraction.
    await control.yieldIfDue(true)
    const evaluatedAt = now()
    // Mesh extraction (normals/BVH/edges) is often the dominant cost — a
    // superseded request must not pay it in full before the newest starts.
    if (options.shouldAbort?.()) throw new AbortedError()
    const sections = shapes.filter(shape => shape.dimension === 2)
    if (sections.length) warn(ctx, `${sections.length} top-level 2D object(s) are not displayed; wrap them in linear_extrude() or rotate_extrude()`)
    const solids = shapes.filter((shape): shape is Shape3D => shape.dimension === 3 && !ctx.kernel.isEmpty(shape.geometry))
    const meshes: MeshData[] = []
    let volume = 0
    let surfaceArea = 0
    let triangleCount = 0
    const exactRoots: NonNullable<ParseResult['exactSolids']>['roots'][number][] = []
    for (const [index, shape] of solids.entries()) {
      if (options.shouldAbort?.()) throw new AbortedError()
      // Resolved here, while the handle is still alive and before its mesh is extracted.
      if (exactRecorder) {
        const name = `Body ${index + 1}`
        const resolved = exactRecorder.recording.resolve(shape.geometry)
        exactRoots.push('id' in resolved ? { name, id: resolved.id } : { name, inexact: resolved.inexact })
      }
      const analysis = options.shouldAbort || options.onYield
        ? await ctx.kernel.analyzeSolidCooperatively(shape.geometry, async () => {
          control.poll()
          await control.yieldIfDue()
        })
        : ctx.kernel.analyzeSolid(shape.geometry)
      volume += analysis.volume
      surfaceArea += analysis.surfaceArea
      const mesh = analysis.mesh
      await control.yieldIfDue()
      triangleCount += mesh.numTri
      if (triangleCount > MAX_TRIANGLES) evaluationError(ctx, 0, `Rendered model exceeds ${MAX_TRIANGLES.toLocaleString()} triangles`)
      if (mesh.numProp < 6) evaluationError(ctx, 0, 'Geometry kernel did not produce normals')
      let vertices: Float32Array
      if (mesh.numProp === 6 && mesh.vertProperties instanceof Float32Array && isExclusiveView(mesh.vertProperties, mesh.numVert * 6)) {
        // The kernel copied this array out for this analysis alone; publish it
        // without another copy. The Worker protocol requires exclusive buffers.
        vertices = mesh.vertProperties
      } else {
        vertices = new Float32Array(mesh.numVert * 6)
        for (let vertex = 0; vertex < mesh.numVert; vertex++) {
          const sourceOffset = vertex * mesh.numProp
          const targetOffset = vertex * 6
          for (let channel = 0; channel < 6; channel++) vertices[targetOffset + channel] = mesh.vertProperties[sourceOffset + channel]
          if ((vertex & 0x3fff) === 0x3fff) await control.yieldIfDue()
        }
      }
      const indices = isExclusiveView(mesh.triVerts, mesh.triVerts.length) ? mesh.triVerts : new Uint32Array(mesh.triVerts)
      const { bvh, semanticEdges } = analysis
      const provenance: MeshProvenanceRun[] = []
      for (let run = 0; run < mesh.runOriginalID.length; run++) {
        const triangleStart = (mesh.runIndex[run] ?? 0) / 3
        const triangleEnd = (mesh.runIndex[run + 1] ?? mesh.triVerts.length) / 3
        if (triangleEnd <= triangleStart) continue
        const originalId = mesh.runOriginalID[run]
        provenance.push({
          triangleStart,
          triangleEnd,
          source: sourceReferences.get(originalId) ?? null,
          backside: ((mesh.runFlags[run] ?? 0) & 1) !== 0,
        })
        if ((run & 0x1fff) === 0x1fff) await control.yieldIfDue()
      }
      if (provenance.length === 0 && mesh.numTri > 0) {
        provenance.push({ triangleStart: 0, triangleEnd: mesh.numTri, source: null, backside: false })
      }
      meshes.push({
        entityId: shape.entityId,
        geometryAssetId: geometryAssetId(vertices, indices),
        vertices,
        indices,
        bvh,
        edgeIndices: semanticEdges.indices,
        color: [...shape.color],
        transform: identity(),
        faceIds: isExclusiveView(mesh.faceID, mesh.faceID.length) ? mesh.faceID : new Uint32Array(mesh.faceID),
        provenance,
        topology: semanticEdges.diagnostics,
      })
      await control.yieldIfDue()
    }
    const analyzedAt = now()
    return {
      meshes, warnings, volume, surfaceArea, quality, reduced: reduced.value,
      ...(exactRecorder ? { exactSolids: { nodes: exactRecorder.recording.nodes, roots: exactRoots } } : {}),
      timings: {
        parseMs,
        bindMs,
        initializeMs: Math.max(0, initializedAt - parsedAt),
        evaluateMs: Math.max(0, evaluatedAt - initializedAt),
        analyzeMs: Math.max(0, analyzedAt - evaluatedAt),
      },
    }
  } finally {
    kernelSession.dispose()
  }
}
