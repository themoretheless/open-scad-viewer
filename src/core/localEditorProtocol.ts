export const LOCAL_EDITOR_PATH = '/__openscada/mcp-editor'
export const LOCAL_EDITOR_HEADER = 'x-openscada-editor'
export const LOCAL_EDITOR_DEFAULT_PORT = 5173
export const LOCAL_EDITOR_TIMEOUT_MS = 8_000
export const LOCAL_EDITOR_MAX_BYTES = 2 * 1024 * 1024

export function isEditorPort(value: number): boolean {
  return Number.isInteger(value) && value >= 1 && value <= 65_535
}
