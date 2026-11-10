/** Loopback companion client for `printer-cli serve` (no direct printer sockets). */

export const PRINTER_COMPANION_DEFAULT_URL = 'http://127.0.0.1:17890'

export type PrinterVendor =
  | 'bambu'
  | 'moonraker'
  | 'octoprint'
  | 'prusa'
  | 'creality'
  | 'snapmaker'

export interface PrinterCompanionConfig {
  host: string
  access_code?: string
  serial?: string
  api_key?: string
  token?: string
  plate_gcode_path?: string
  verify_start?: boolean
}

export interface DiscoveredCompanionPrinter {
  vendor: string
  display_name?: string | null
  host: string
  port?: number | null
  serial?: string | null
  model?: string | null
}

export interface CompanionSendResult {
  remoteName: string
  verified: boolean
  gcodeState?: string | null
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text()
  try {
    return text ? JSON.parse(text) : null
  } catch {
    throw new Error(`Companion returned non-JSON (${response.status}).`)
  }
}

export async function companionHealth(baseUrl = PRINTER_COMPANION_DEFAULT_URL): Promise<boolean> {
  try {
    const response = await fetch(`${baseUrl.replace(/\/$/, '')}/health`, { method: 'GET' })
    if (!response.ok) return false
    const body = await readJson(response) as { ok?: boolean }
    return body?.ok === true
  } catch {
    return false
  }
}

export async function companionDiscover(baseUrl = PRINTER_COMPANION_DEFAULT_URL): Promise<DiscoveredCompanionPrinter[]> {
  const response = await fetch(`${baseUrl.replace(/\/$/, '')}/discover`, { method: 'GET' })
  const body = await readJson(response) as { printers?: DiscoveredCompanionPrinter[]; error?: string }
  if (!response.ok) throw new Error(body?.error || `Discover failed (${response.status}).`)
  return Array.isArray(body.printers) ? body.printers : []
}

export async function companionSend(
  vendor: PrinterVendor,
  config: PrinterCompanionConfig,
  fileName: string,
  bytesBase64: string,
  baseUrl = PRINTER_COMPANION_DEFAULT_URL,
): Promise<CompanionSendResult> {
  const response = await fetch(`${baseUrl.replace(/\/$/, '')}/send`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ vendor, config, fileName, bytesBase64 }),
  })
  const body = await readJson(response) as CompanionSendResult & { error?: string }
  if (!response.ok) throw new Error(body?.error || `Send failed (${response.status}).`)
  return {
    remoteName: body.remoteName,
    verified: !!body.verified,
    gcodeState: body.gcodeState,
  }
}

export function utf8ToBase64(text: string): string {
  const bytes = new TextEncoder().encode(text)
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}
