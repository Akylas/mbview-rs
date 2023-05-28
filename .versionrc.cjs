const tracker = {
  filename: './src-tauri/Cargo.toml',
  updater: require('./tauriVersioner.cjs')
}

module.exports = {
  bumpFiles: [tracker],
  packageFiles: [tracker]
}