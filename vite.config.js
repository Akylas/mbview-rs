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
export default defineConfig(({ command, mode }) => {
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

    resolve: {
      alias: {
        'mapbox-gl': 'maplibre-gl',
      },
    },
    optimizeDeps: {
      // include: ['geo-three']
    },
    build: {
      outDir: '../build',
      emptyOutDir: true,
      minify: true,
      sourcemap: false,
      target: 'modules',
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
