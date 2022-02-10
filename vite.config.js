import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import sveltePreprocess from 'svelte-preprocess'

const ignoreWarnings = new Set(['a11y-no-onchange', 'a11y-label-has-associated-control', 'a11y-mouse-events-have-key-events', 'a11y-mouse-events-have-key-events']);

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
    minify: false,
    sourcemap: true,
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
    FORCE_MOBILE:false,
    EXTERNAL_APP:false
  }
})
