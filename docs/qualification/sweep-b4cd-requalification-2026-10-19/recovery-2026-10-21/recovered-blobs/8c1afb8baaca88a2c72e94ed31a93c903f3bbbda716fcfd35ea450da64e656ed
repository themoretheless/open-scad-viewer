import { MAX_WORKSPACE_SOURCE_LENGTH } from './workspaceDocument'

const MAX_SHARE_HASH_LENGTH = 1_400_000

export function encodeWorkspaceShare(value: string): string {
  const bytes = new TextEncoder().encode(value)
  let binary = ''
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000))
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function decodeWorkspaceShare(value: string): string {
  if (!/^[A-Za-z0-9_-]*$/.test(value) || value.length % 4 === 1) {
    throw new TypeError('Share payload is not canonical URL-safe base64')
  }
  const normalized = value.replace(/-/g, '+').replace(/_/g, '/')
  const binary = atob(normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '='))
  const bytes = Uint8Array.from(binary, character => character.charCodeAt(0))
  const decoded = new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  if (encodeWorkspaceShare(decoded) !== value) throw new TypeError('Share payload is not canonical')
  return decoded
}

export function readWorkspaceShareHash(hash: string): string | null {
  try {
    if (!hash.startsWith('#code=') || hash.length > MAX_SHARE_HASH_LENGTH) return null
    const source = decodeWorkspaceShare(hash.slice(6))
    return source.length <= MAX_WORKSPACE_SOURCE_LENGTH ? source : null
  } catch {
    return null
  }
}
