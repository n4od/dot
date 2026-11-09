import { defineConfig } from 'vitest/config'

/*
 * A second, one-off Vitest config for tooling that must *not* be part of the
 * normal suite: `pnpm test` collects `src/**\/*.test.ts` only, while this runs
 * `tools/golden/dump.ts` to regenerate the engine's golden-frame fixtures.
 */
export default defineConfig({
  test: {
    environment: 'node',
    include: ['tools/golden/**/*.ts']
  }
})