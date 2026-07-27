const { createDefaultPreset } = require("ts-jest");

const tsJestTransformCfg = createDefaultPreset().transform;

/** @type {import("jest").Config} **/
module.exports = {
  testEnvironment: "node",
  transform: { '^.+\\.(t|j)s$': ['ts-jest', { tsconfig: 'tsconfig.spec.json' }] },
  transformIgnorePatterns: [
    // Tell Jest to compile uuid, but keep ignoring all other node_modules
    '/node_modules/(?!uuid)/'
  ],
};