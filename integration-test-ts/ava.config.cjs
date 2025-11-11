require('util').inspect.defaultOptions.depth = 10; // Increase AVA's printing depth

module.exports = {
  timeout: '300000',
  files: ['src/**/*.ava.ts'],
  failWithoutAssertions: false,
  extensions: {
    ts: 'commonjs',
  },
  require: ['ts-node/register'],
};
