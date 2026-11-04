import { existsSync } from 'node:fs'
import { expect, test } from 'bun:test'

test('installable app uses Matis branding and existing icons', async () => {
  const manifest = await Bun.file(
    new URL('../public/manifest.json', import.meta.url),
  ).json()
  expect(manifest.name).toBe('Matis')
  expect(manifest.short_name).toBe('Matis')
  expect(manifest.icons.length).toBeGreaterThan(0)
  for (const icon of manifest.icons)
    expect(existsSync(new URL(`../public/${icon.src}`, import.meta.url))).toBe(
      true,
    )
})
