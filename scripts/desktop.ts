import { dirname, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const directory = dirname(process.execPath)
const dashboard = process.platform === 'darwin'
  ? resolve(directory, '../Resources/dashboard/server/index.mjs')
  : resolve(directory, '../lib/matis/dashboard/server/index.mjs')

const migration = Bun.spawnSync([resolve(directory, 'matis-migrate')], {
  stdout: 'inherit',
  stderr: 'inherit',
})
if (migration.exitCode !== 0) throw new Error('Database migration failed')

process.env.NITRO_HOST = '127.0.0.1'
process.env.NITRO_PORT = '43127'
await import(pathToFileURL(dashboard).href)

const url = 'http://127.0.0.1:43127'
for (let attempt = 0; attempt < 50; attempt++) {
  const response = await fetch(url).catch(() => null)
  if (response?.ok) {
    const result = Bun.spawnSync([process.platform === 'darwin' ? 'open' : 'xdg-open', url])
    if (result.exitCode !== 0) throw new Error('Could not open the default browser')
    break
  }
  if (attempt === 49) throw new Error('Dashboard did not start')
  await Bun.sleep(100)
}
