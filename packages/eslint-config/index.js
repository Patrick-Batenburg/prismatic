import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import svelteParser from "svelte-eslint-parser";
import prettier from "eslint-config-prettier";
import globals from "globals";

/** @param {{ tsconfigPath?: string }} options */
export function createConfig({ tsconfigPath } = {}) {
  return tseslint.config(
    eslint.configs.recommended,
    ...tseslint.configs.recommended,
    ...svelte.configs.recommended,
    prettier,
    ...svelte.configs.prettier,
    {
      languageOptions: {
        globals: {
          ...globals.browser,
          ...globals.node,
        },
      },
    },
    {
      files: ["**/*.svelte"],
      languageOptions: {
        parser: svelteParser,
        parserOptions: {
          parser: tseslint.parser,
          ...(tsconfigPath && { project: tsconfigPath }),
        },
      },
    },
    {
      files: ["**/*.ts"],
      languageOptions: {
        parser: tseslint.parser,
        ...(tsconfigPath && {
          parserOptions: { project: tsconfigPath },
        }),
      },
    },
    {
      rules: {
        "@typescript-eslint/no-unused-vars": [
          "error",
          { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
        ],
        "@typescript-eslint/no-explicit-any": "error",
        "no-console": ["error", { allow: ["warn", "error"] }],
        "svelte/no-at-html-tags": "error",
        "svelte/require-each-key": "error",
        "svelte/no-navigation-without-resolve": "off",
        "svelte/no-unused-svelte-ignore": "error",
        "svelte/prefer-svelte-reactivity": "error",
      },
    },
    {
      ignores: [
        "**/node_modules/**",
        "**/build/**",
        "**/.svelte-kit/**",
        "**/target/**",
        "**/.turbo/**",
      ],
    },
  );
}

export default createConfig();
