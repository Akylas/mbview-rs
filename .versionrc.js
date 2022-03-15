const tracker = {
    filename: './src-tauri/Cargo.toml',
    updater: require('./tauriVersioner')
  }
  
  module.exports = {
    // bumpFiles: [tracker],
    packageFiles: [tracker]
  }