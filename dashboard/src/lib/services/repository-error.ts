export class RepositoryError extends Error {
  constructor(
    message: string,
    public readonly status: 400 | 404 | 409,
  ) {
    super(message)
    this.name = 'RepositoryError'
  }
}

export const requiredText = (
  value: unknown,
  field: string,
  maxLength: number,
) => {
  if (typeof value !== 'string' || !value.trim())
    throw new RepositoryError(`${field} is required`, 400)
  const text = value.trim()
  if (text.length > maxLength)
    throw new RepositoryError(
      `${field} must be at most ${maxLength} characters`,
      400,
    )
  return text
}
