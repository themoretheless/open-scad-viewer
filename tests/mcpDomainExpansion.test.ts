import { createServer } from 'node:http'
import { afterEach, beforeAll, describe, expect, it } from 'vitest'
import { InMemoryTransport, type JSONRPCMessage } from '@modelcontextprotocol/server'
import { LOCAL_EDITOR_HEADER } from '../src/core/localEditorProtocol'
import { exportDirectStepV9 } from '../src/kernel/cadNurbsStep'
import { createBrepBox } from '../src/kernel/geometry/brep'
import { warmGeometryKernel } from '../src/kernel/geometry/kernel'
import { createRushGraphMcpServer } from '../src/mcp/createRushGraphServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import { createRushGraphHttpServer } from '../src/mcp/rushGraphHttp'
import { parseMcpCliOptions } from '../src/mcp/server'
import { ALL_MCP_TOOLSET_PROFILES, parseMcpToolsetProfiles, type McpToolsetProfile } from '../src/mcp/toolsetProfiles'

interface JsonRpcResponse {
  jsonrpc: '2.0'
  id: number
  result?: Record<string, unknown>
  error?: { code: number; message: string }
}

const closeables: Array<() => Promise<void>> = []

beforeAll(async () => {
  await warmGeometryKernel()
})

afterEach(async () => {
  for (const close of closeables.splice(0).reverse()) {
    await close()
  }
})

async function connectTestServer(
  options: {
    localEditor?: boolean
    toolsetProfiles?: ReadonlySet<McpToolsetProfile>
  } = {},
) {
  const store = await DuckDbModelStore.open(':memory:')
  const server = createRushGraphMcpServer({
    store,
    ...(options.localEditor !== undefined ? { localEditor: options.localEditor } : {}),
    ...(options.toolsetProfiles !== undefined ? { toolsetProfiles: options.toolsetProfiles } : {}),
  })
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
  const pending = new Map<number, (response: JsonRpcResponse) => void>()
  clientTransport.onmessage = message => {
    if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
      pending.get(message.id)?.(message as JsonRpcResponse)
      pending.delete(message.id)
    }
  }
  await clientTransport.start()
  await server.connect(serverTransport)
  let nextId = 1
  const request = async (method: string, params: Record<string, unknown> = {}) => {
    const id = nextId++
    const responsePromise = new Promise<JsonRpcResponse>((resolve, reject) => {
      const timeout = setTimeout(() => {
        pending.delete(id)
        reject(new Error(`Timed out waiting for ${method}`))
      }, 20_000)
      pending.set(id, value => {
        clearTimeout(timeout)
        resolve(value)
      })
    })
    await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
    const response = await responsePromise
    if (response.error) throw new Error(`${response.error.code}: ${response.error.message}`)
    return response.result ?? {}
  }
  await request('initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'mcp-domain-expansion-test', version: '1.0.0' },
  })
  await clientTransport.send({ jsonrpc: '2.0', method: 'notifications/initialized' } as JSONRPCMessage)

  closeables.push(async () => {
    await server.close()
    await clientTransport.close()
    await store.close()
  })

  const callTool = async (name: string, args: Record<string, unknown> = {}) =>
    (await request('tools/call', { name, arguments: args })) as {
      isError?: boolean
      structuredContent: Record<string, unknown>
      content: Array<{
        type: string
        text?: string
        resource?: { uri: string; mimeType?: string; blob?: string; text?: string }
      }>
    }

  return { request, callTool }
}

const OPEN_TOP_CUBE_OBJ = [
  'v 0 0 0',
  'v 10 0 0',
  'v 10 10 0',
  'v 0 10 0',
  'v 0 0 10',
  'v 10 0 10',
  'v 10 10 10',
  'v 0 10 10',
  'f 1 3 2',
  'f 1 4 3',
  'f 1 2 6',
  'f 1 6 5',
  'f 2 3 7',
  'f 2 7 6',
  'f 3 4 8',
  'f 3 8 7',
  'f 4 1 5',
  'f 4 5 8',
].join('\n')

const TESSELLATED_STEP_TET = [
  'ISO-10303-21;',
  'HEADER;',
  "FILE_DESCRIPTION(('Tessellated'),'2;1');",
  "FILE_NAME('tet.stp','2026-01-01T00:00:00',(''),(''),'','','');",
  "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));",
  'ENDSEC;',
  'DATA;',
  "#1=CARTESIAN_POINT_LIST_3D('',((0.,0.,0.),(10.,0.,0.),(0.,10.,0.),(0.,0.,10.)));",
  "#2=TRIANGULATED_FACE_SET('',#1,$,(),((1,3,2),(1,2,4),(2,3,4),(3,1,4)),());",
  "#3=TESSELLATED_SHAPE_REPRESENTATION('',(#2),$);",
  'ENDSEC;',
  'END-ISO-10303-21;',
].join('\n')

describe('MCP domain expansion & profile filtering', () => {
  it('filters tools, resources, and prompts by --profiles and OPENSCAD_VIEWER_MCP_PROFILES', async () => {
    expect(parseMcpToolsetProfiles('all')).toBe(ALL_MCP_TOOLSET_PROFILES)
    expect(parseMcpCliOptions(['--profiles', 'cam,cae'], {}, '/tmp').toolsetProfiles).toEqual(new Set(['cam', 'cae']))
    expect(
      parseMcpCliOptions([], { OPENSCAD_VIEWER_MCP_PROFILES: 'drawing,exchange' }, '/tmp').toolsetProfiles,
    ).toEqual(new Set(['drawing', 'exchange']))
    expect(() => parseMcpCliOptions(['--profiles'], {}, '/tmp')).toThrow('--profiles requires')
    expect(() => parseMcpCliOptions(['--profiles', 'invalid_profile'], {}, '/tmp')).toThrow('Unknown MCP profile')

    const { request } = await connectTestServer({ toolsetProfiles: new Set(['cam', 'cae']) })
    const tools = ((await request('tools/list')) as { tools: Array<{ name: string }> }).tools.map(t => t.name).sort()
    expect(tools).toEqual([
      'cae_frame_analyze',
      'cae_printed_solid_analyze',
      'cae_truss_analyze',
      'cam_inspect_gcode',
      'cam_laser_section_plan',
      'cam_print_readiness',
      'cam_slice_gcode',
    ])
    const resources = ((await request('resources/list')) as { resources: Array<{ uri: string }> }).resources.map(
      r => r.uri,
    )
    expect(resources).toEqual(['cae://guide'])
    const prompts = ((await request('prompts/list')) as { prompts: Array<{ name: string }> }).prompts
      .map(p => p.name)
      .sort()
    expect(prompts).toEqual(['cae_structural_optimization', 'cam_print_and_slice_workflow'])
  })

  it('supports openscad_delete_model and non-destructive openscad_editor_status', async () => {
    const mockBridge = createServer((_req, res) => {
      res.setHeader(LOCAL_EDITOR_HEADER, 'bridge')
      res.writeHead(400).end()
    })
    await new Promise<void>(resolve => mockBridge.listen(0, '127.0.0.1', resolve))
    const port = (mockBridge.address() as { port: number }).port
    const closeBridge = () => new Promise<void>(resolve => mockBridge.close(() => resolve()))
    closeables.push(closeBridge)

    const { callTool } = await connectTestServer({ localEditor: true })
    const saved = await callTool('openscad_save_model', { id: 'exp:temp', name: 'Temp', source: 'cube(2);' })
    expect(saved.isError).not.toBe(true)
    const deleted = await callTool('openscad_delete_model', { model_id: 'exp:temp' })
    expect(deleted.structuredContent).toEqual({ model_id: 'exp:temp', deleted: true })
    const deletedAgain = await callTool('openscad_delete_model', { model_id: 'exp:temp' })
    expect(deletedAgain.structuredContent).toEqual({ model_id: 'exp:temp', deleted: false })
    const missing = await callTool('openscad_get_model', { id: 'exp:temp' })
    expect(missing.isError).toBe(true)

    const readyStatus = await callTool('openscad_editor_status', { port })
    expect(readyStatus.isError).not.toBe(true)
    expect(readyStatus.structuredContent).toEqual({ port, status: 'tab_ready' })

    await closeBridge()
    const unreachableStatus = await callTool('openscad_editor_status', { port })
    expect(unreachableStatus.isError).not.toBe(true)
    expect(unreachableStatus.structuredContent).toEqual({ port, status: 'unreachable' })
  })

  it('executes direct modeling, 2D sketch solver, and B-rep contact/distance checks', async () => {
    const { request, callTool } = await connectTestServer({ toolsetProfiles: new Set(['direct']) })
    const schemaRes = (await request('resources/read', { uri: 'direct://schema' })) as {
      contents: Array<{ text: string }>
    }
    expect(JSON.parse(schemaRes.contents[0]!.text).schema).toBe('openscad-viewer/direct-document-schema-v1')

    const created = await callTool('direct_create_document', { units: 'mm' })
    expect(created.isError).not.toBe(true)
    const tx = await callTool('openscad_direct_transaction', {
      document: created.structuredContent.document,
      script: JSON.stringify({
        version: 1,
        actions: [
          {
            kind: 'primitive',
            options: {
              kind: 'box',
              id: 'b1',
              name: 'Box',
              size: 10,
              topRadius: 1,
              innerRadius: 0.5,
              round: 'exact',
              segments: 1,
            },
          },
        ],
      }),
    })
    expect(tx.isError).not.toBe(true)
    const inspected = await callTool('direct_inspect_document', { document: tx.structuredContent.document })
    expect(inspected.structuredContent.body_count).toBe(1)

    const solved = await callTool('sketch_solve', {
      points: [
        { id: 'p0', position: [0.1, -0.1] },
        { id: 'p1', position: [9.5, 0.2] },
        { id: 'p2', position: [9.8, 4.8] },
        { id: 'p3', position: [-0.1, 5.2] },
      ],
      constraints: [
        { id: 'c_fix', kind: 'fix', point: 'p0', at: [0, 0] },
        { id: 'c_h1', kind: 'horizontal', a: 'p0', b: 'p1' },
        { id: 'c_h2', kind: 'horizontal', a: 'p3', b: 'p2' },
        { id: 'c_v1', kind: 'vertical', a: 'p0', b: 'p3' },
        { id: 'c_v2', kind: 'vertical', a: 'p1', b: 'p2' },
        { id: 'c_w', kind: 'distance', a: 'p0', b: 'p1', value: 10 },
        { id: 'c_h', kind: 'distance', a: 'p0', b: 'p3', value: 5 },
      ],
    })
    expect(solved.isError).not.toBe(true)
    expect(solved.structuredContent.status).toBe('solved')

    const boxA = {
      nurbs_document: {
        language: 'rush/nurbs-1',
        units: 'mm',
        parameters: [],
        nodes: [{ id: 'b', op: 'brep_box', min: [0, 0, 0], max: [2, 2, 2] }],
        root: 'b',
      },
    }
    const boxB = {
      nurbs_document: {
        language: 'rush/nurbs-1',
        units: 'mm',
        parameters: [],
        nodes: [{ id: 'b', op: 'brep_box', min: [5, 0, 0], max: [7, 2, 2] }],
        root: 'b',
      },
    }
    const contacts = await callTool('brep_inspect_contacts', { model: boxA })
    expect(contacts.isError).not.toBe(true)
    expect(contacts.structuredContent.valid).toBe(true)
    expect(contacts.structuredContent.face_count).toBe(6)

    const dist = await callTool('brep_solid_distance', { a: boxA, b: boxB })
    expect(dist.isError).not.toBe(true)
    expect(dist.structuredContent.certified).toBe(true)
    expect(dist.structuredContent.relation).toBe('separated-volumes')
    expect(Number(dist.structuredContent.distance_lower_mm)).toBeGreaterThan(2.9)
  }, 20_000)

  it('imports/exports STEP and IGES, inspects meshes, and repairs non-watertight meshes', async () => {
    const { callTool } = await connectTestServer({ toolsetProfiles: new Set(['exchange']) })
    const boxStep = exportDirectStepV9(createBrepBox([0, 0, 0], [10, 10, 10])).text
    const stepBrep = await callTool('cad_import_step', {
      step_text: boxStep,
      include_mesh_preview: true,
      export_mesh_format: 'stl',
    })
    expect(stepBrep.isError).not.toBe(true)
    expect(stepBrep.structuredContent.mode).toBe('direct-brep-step')
    expect((stepBrep.structuredContent.topology as { face_count: number }).face_count).toBe(6)
    expect(stepBrep.content.some(c => c.type === 'resource' && c.resource?.mimeType === 'model/stl')).toBe(true)

    const stepTess = await callTool('cad_import_step', {
      step_text: TESSELLATED_STEP_TET,
      include_mesh_preview: true,
    })
    expect(stepTess.isError).not.toBe(true)
    expect(stepTess.structuredContent.mode).toBe('tessellated-step')

    const boxDoc = {
      language: 'rush/nurbs-1',
      units: 'mm',
      parameters: [],
      nodes: [{ id: 'b', op: 'brep_box', min: [0, 0, 0], max: [4, 4, 4] }],
      root: 'b',
    }
    const igesExp = await callTool('cad_export_iges', { model: { nurbs_document: boxDoc } })
    expect(igesExp.isError).not.toBe(true)
    const igesImp = await callTool('cad_import_iges', { iges_text: String(igesExp.structuredContent.iges_text) })
    expect(igesImp.isError).not.toBe(true)
    expect((igesImp.structuredContent.topology as { face_count: number }).face_count).toBe(6)

    const openObjB64 = Buffer.from(OPEN_TOP_CUBE_OBJ, 'utf8').toString('base64')
    const inspectOpen = await callTool('mesh_inspect', {
      target: { file_name: 'open-box.obj', data_base64: openObjB64 },
    })
    expect(inspectOpen.isError).not.toBe(true)
    expect(inspectOpen.structuredContent.watertight).toBe(false)

    const repaired = await callTool('mesh_repair', {
      target: { file_name: 'open-box.obj', data_base64: openObjB64 },
      mode: 'full',
      export_format: 'stl',
    })
    expect(repaired.isError).not.toBe(true)
    expect(repaired.structuredContent.watertight).toBe(true)
    expect(repaired.structuredContent.filled_holes).toBeGreaterThanOrEqual(1)
    expect(Number(repaired.structuredContent.exported_byte_length)).toBeGreaterThan(84)
  }, 20_000)

  it('generates SVG + PDF drawings, ISO 286 fits, CAM G-code, laser plans, CAE solves, and serves HTTP /mcp', async () => {
    const { request, callTool } = await connectTestServer({ toolsetProfiles: new Set(['drawing', 'cam', 'cae']) })

    const drawing = await callTool('cad_drawing_sheet', {
      target: { source: 'difference() { cube([20,20,10]); translate([5,5,-1]) cylinder(h=12, r=3, $fn=24); }' },
      title: 'Block',
      section: { view: 'top', offset: 5 },
      include_pdf: true,
    })
    expect(drawing.isError).not.toBe(true)
    expect(String(drawing.structuredContent.svg)).toContain('<svg')
    const pdfBlob = drawing.content.find(c => c.resource?.mimeType === 'application/pdf')?.resource?.blob
    expect(Buffer.from(pdfBlob!, 'base64').toString('utf8', 0, 5)).toBe('%PDF-')

    const fit = await callTool('cad_iso286_fit', { nominal_diameter_mm: 20, hole_zone: 'H7', shaft_zone: 'g6' })
    expect(fit.isError).not.toBe(true)
    expect(fit.structuredContent.fit_type).toBe('clearance')

    const openTriB64 = Buffer.from('v 0 0 0\nv 5 0 0\nv 0 5 0\nf 1 2 3\n', 'utf8').toString('base64')
    const notReady = await callTool('cam_print_readiness', {
      target: { file_name: 'tri.obj', data_base64: openTriB64 },
    })
    expect(notReady.isError).not.toBe(true)
    expect(notReady.structuredContent.ready).toBe(false)
    expect(notReady.structuredContent.error_code).toBe('PRINT_NOT_WATERTIGHT')

    const sliced = await callTool('cam_slice_gcode', { target: { source: 'cube([10,10,4]);' }, mode: 'preview' })
    expect(sliced.isError).not.toBe(true)
    const gcodeText = sliced.content.find(c => c.resource?.uri === 'cam://gcode/output.gcode')?.resource?.text ?? ''
    const inspectedGcode = await callTool('cam_inspect_gcode', { gcode: gcodeText })
    expect(inspectedGcode.isError).not.toBe(true)
    expect(inspectedGcode.structuredContent.native).toBe(true)
    expect(Number(inspectedGcode.structuredContent.layers)).toBeGreaterThan(0)

    const laser = await callTool('cam_laser_section_plan', {
      target: { source: 'cube([20,10,4]);' },
      z_mm: 2,
      laser_mode_confirmed: true,
    })
    expect(laser.isError).not.toBe(true)
    expect(laser.structuredContent.gcode_emitted).toBe(true)

    const guide = JSON.parse(
      ((await request('resources/read', { uri: 'cae://guide' })) as { contents: Array<{ text: string }> }).contents[0]!
        .text,
    )
    const truss = await callTool('cae_truss_analyze', {
      ...guide.examples.truss,
      buckling_modes: 1,
      axial_limits: { tension_mpa: 250, compression_mpa: 250, safety_factor: 1.5 },
    })
    expect(truss.isError).not.toBe(true)
    expect(Number(truss.structuredContent.max_deflection_mm)).toBeGreaterThan(0)

    const frame = await callTool('cae_frame_analyze', guide.examples.frame)
    expect(frame.isError).not.toBe(true)
    expect(Number(frame.structuredContent.max_deflection_mm)).toBeGreaterThan(0)

    const bonded = await callTool('cae_printed_solid_analyze', guide.examples.printed_solid)
    expect(bonded.isError).not.toBe(true)
    expect(bonded.structuredContent.limit_reached).toBe(false)

    const httpServer = createRushGraphHttpServer({ maxBodyBytes: 4 * 1024 * 1024 })
    await new Promise<void>(resolve => httpServer.listen(0, '127.0.0.1', resolve))
    closeables.push(async () => {
      httpServer.closeAllConnections()
      await new Promise<void>(resolve => httpServer.close(() => resolve()))
    })
    const httpPort = (httpServer.address() as { port: number }).port
    const httpRes = await fetch(`http://127.0.0.1:${httpPort}/mcp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json, text/event-stream',
        'MCP-Protocol-Version': '2025-11-25',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: { name: 'cad_iso286_fit', arguments: { nominal_diameter_mm: 20, hole_zone: 'H7', shaft_zone: 'p6' } },
      }),
    })
    const httpJson = (await httpRes.json()) as {
      result: { isError?: boolean; structuredContent: { fit_type: string } }
    }
    expect(httpJson.result.isError).not.toBe(true)
    expect(httpJson.result.structuredContent.fit_type).toBe('interference')
  }, 25_000)
})
