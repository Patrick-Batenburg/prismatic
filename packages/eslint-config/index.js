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
          ...(tsconfigPath && {
            project: tsconfigPath,
            extraFileExtensions: [".svelte"],
          }),
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
        // TypeScript
        "@typescript-eslint/no-unused-vars": [
          "error",
          { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
        ],
        "@typescript-eslint/no-explicit-any": "error",
        "@typescript-eslint/consistent-type-assertions": [
          "error",
          { assertionStyle: "never" },
        ],
        "@typescript-eslint/consistent-type-imports": [
          "error",
          { prefer: "type-imports", fixStyle: "inline-type-imports" },
        ],
        "@typescript-eslint/naming-convention": [
          "error",
          { selector: "typeLike", format: ["PascalCase"] },
        ],

        // Core JS
        eqeqeq: ["error", "always"],
        "prefer-const": "error",
        "no-param-reassign": ["error", { props: false }],
        "no-nested-ternary": "error",
        "max-depth": ["error", 4],
        "no-console": ["error", { allow: ["warn", "error"] }],

        // Svelte
        "svelte/no-at-html-tags": "error",
        "svelte/require-each-key": "error",
        "svelte/no-navigation-without-resolve": "off",
        "svelte/no-unused-svelte-ignore": "error",
        "svelte/prefer-svelte-reactivity": "error",
      },
    },
    // Type-aware rules (require tsconfig project)
    ...(tsconfigPath
      ? [
          {
            files: ["**/*.ts", "**/*.svelte"],
            rules: {
              "@typescript-eslint/prefer-nullish-coalescing": "error",
              "@typescript-eslint/prefer-optional-chain": "error",
              "@typescript-eslint/switch-exhaustiveness-check": "error",
              "@typescript-eslint/no-unnecessary-condition": "error",
              "@typescript-eslint/strict-boolean-expressions": [
                "error",
                {
                  allowNullableBoolean: true,
                  allowNullableString: true,
                  allowNullableNumber: false,
                  allowNullableObject: true,
                },
              ],
              "@typescript-eslint/no-floating-promises": "error",
              "@typescript-eslint/no-misused-promises": [
                "error",
                { checksVoidReturn: { attributes: false } },
              ],
            },
          },
        ]
      : []),
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
