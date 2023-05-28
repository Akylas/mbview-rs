// eslint-disable-next-line @typescript-eslint/no-var-requires
const TOML = require('@tauri-apps/toml');

module.exports.readVersion = function (contents) {
    return TOML.parse(contents).package.version;
};
module.exports.writeVersion = function (contents, version) {
    const newData = TOML.parse(contents);
    newData.package.version = version;
    return TOML.stringify(newData);
};
