import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import sveltePreprocess from 'svelte-preprocess';
import { readdirSync } from 'fs';
import { join } from 'path';
import config from './package.json';
const ignoreWarnings = new Set([
  'a11y-no-onchange',
  'a11y-label-has-associated-control',
  'a11y-mouse-events-have-key-events',
  'a11y-mouse-events-have-key-events',
]);

// eslint-disable-next-line @typescript-eslint/no-var-requires
// const config = require('./package.json');
export default defineConfig(({ mode }) => {
  console.log('defineConfig', mode);
  const locales = readdirSync(join('src', 'i18n'))
    .filter((s) => s.endsWith('.json'))
    .map((s) => s.replace('.json', ''));
  const production = mode === 'production';
  console.log('mode', mode);
  return {
    root: './src',
    base: './', // use relative paths
    publicDir: '../public',
    clearScreen: false,
    server: {
      port: 3000,
      strictPort: true,
    },

    // to make use of `TAURI_ENV_PLATFORM`, `TAURI_ENV_ARCH`, `TAURI_ENV_FAMILY`, `TAURI_ENV_PLATFORM_VERSION`,
    // `TAURI_ENV_PLATFORM_TYPE` and `TAURI_ENV_DEBUG` env variables (renamed from `TAURI_*` in Tauri 2.0)
    envPrefix: ['VITE_', 'TAURI_ENV_*'],
    build: {
      outDir: '../build',
      emptyOutDir: true,
      // tauri supports es2021; safari 14 is the floor for BigInt literals, which the MLT decoder uses
      target: ['es2021', 'chrome97', 'safari14'],
      // don't minify for debug builds
      minify: !process.env.TAURI_ENV_DEBUG && 'esbuild',
      // produce sourcemaps for debug builds
      sourcemap: !!process.env.TAURI_ENV_DEBUG,
    },
    plugins: [
      svelte({
        onwarn(warning, defaultHandler) {
          // don't warn on <marquee> elements, cos they're cool
          if (ignoreWarnings.has(warning.code)) return;

          // handle all other warnings normally
          defaultHandler(warning);
        },
        preprocess: sveltePreprocess(),
      }),
    ],
    define: {
      SUPPORTED_LOCALES: JSON.stringify(locales),
      REPO_URL: `"${config.homepage}"`,
      PRODUCTION: production,
    },
  };
});
