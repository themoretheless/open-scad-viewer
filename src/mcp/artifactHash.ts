import { createHash } from 'node:crypto'

export function dataSha256(data: Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}
