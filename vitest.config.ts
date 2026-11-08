import { defineConfig } from 'vitest/config'

/*
 * Only `src/bot/` has tests: the engine is framework-free and DOM-free, so the
 * whole suite runs in a plain Node environment in about a second. There is no
 * Vue plugin here on purpose — no test in this repo imports a `.vue` file, and
 * adding the plugin would only slow collection down.
 *
 * `src/bot/*.test.ts` is a copy of the upstream suites from `jeremy-prt/bloub`
 * at b4bb3c1, kept alongside the engine so the copy stays honest.
 */
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts']
  }
})
