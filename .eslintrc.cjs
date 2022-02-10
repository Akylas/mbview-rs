const ignoreWarnings = new Set(['a11y-no-onchange', 'a11y-label-has-associated-control', 'a11y-mouse-events-have-key-events', 'a11y-mouse-events-have-key-events']);
module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended', 'prettier'],
  plugins: ['svelte3', '@typescript-eslint'],
  overrides: [{ files: ['*.svelte'], processor: 'svelte3/svelte3' }],
  settings: {
    'svelte3/ignore-warnings': (w) => ignoreWarnings.has(w && w.code),
    'svelte3/typescript': true,
    'svelte3/ignore-styles': (attributes) => {
      // https://github.com/sveltejs/eslint-plugin-svelte3/issues/10
      return attributes && attributes.lang && attributes.lang !== 'css'
    },
  },
  parserOptions: {
    sourceType: 'module',
    ecmaVersion: 2019,
  },
  env: {
    browser: true,
    es2017: true,
    node: true,
  },
  rules:{
    '@typescript-eslint/ban-ts-comment': 'off',
    '@typescript-eslint/no-explicit-any': 'off',
  }
}
