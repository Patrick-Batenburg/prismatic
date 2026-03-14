import { createConfig } from "@prismatic/eslint-config";

export default [
  { ignores: ["vitest.config.ts"] },
  ...createConfig({ tsconfigPath: "./tsconfig.json" }),
];
