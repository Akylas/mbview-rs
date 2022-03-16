import { defineConfig } from 'vite'
import { readdirSync } from 'fs'
import { join } from 'path'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import sveltePreprocess from 'svelte-preprocess'

const ignoreWarnings = new Set(['a11y-no-onchange', 'a11y-label-has-associated-control', 'a11y-mouse-events-have-key-events', 'a11y-mouse-events-have-key-events']);

// eslint-disable-next-line @typescript-eslint/no-var-requires
const config = require('./package.json');

const locales = readdirSync(join('src', 'i18n'))
        .filter((s) => s.endsWith('.json'))
        .map((s) => s.replace('.json', ''));

export default defineConfig({
  root: './src',
  base: './', // use relative paths
  publicDir: '../public',
  clearScreen: false,
  server: {
    port: 3000,
    strictPort: true,
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
    FORCE_MOBILE:false,
    EXTERNAL_APP:false
  }
})
