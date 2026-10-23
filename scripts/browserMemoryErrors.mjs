export class BrowserMemoryQualificationError extends Error {
  constructor(code, message, details = {}) {
    super(message)
    this.name = 'BrowserMemoryQualificationError'
    this.code = code
    this.details = Object.freeze({ ...details })
  }
}

export function serializedError(error) {
  if (error instanceof AggregateError) {
    return {
      name: error.name,
      code: 'E_AGGREGATE',
      message: error.message,
      details: { errors: [...error.errors].map(serializedError) },
    }
  }
  if (error instanceof BrowserMemoryQualificationError) {
    return {
      name: error.name,
      code: error.code,
      message: error.message,
      details: error.details,
    }
  }
  return {
    name: error instanceof Error ? error.name : 'UnknownError',
    code: 'E_UNEXPECTED',
    message: error instanceof Error ? error.message : String(error),
    details: {},
  }
}
