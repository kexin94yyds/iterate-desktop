import antfu from '@antfu/eslint-config'

export default antfu({
  vue: true,
  typescript: true,
  ignores: [
    'browser-extension/**',
    'sw.js',
    'vscode-extension/**',
    'vscode-extension-windows/**',
    'website/**',
  ],
  rules: {
    'no-console': 'off',
  },
})
