const cssnano = require('cssnano');
const mode = process.env.NODE_ENV;
const production = mode === 'production';

const config = {
	plugins: [
		production &&
			cssnano({
				preset: 'default'
			})
	]
};

module.exports = config;