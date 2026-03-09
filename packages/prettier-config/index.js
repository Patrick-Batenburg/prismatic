/** @type {import("prettier").Config} */
export default {
  plugins: ["prettier-plugin-svelte"],
  overrides: [{ files: "*.svelte", options: { parser: "svelte" } }],
  semi: true,
  singleQuote: false,
  trailingComma: "all",
  printWidth: 100,
  tabWidth: 2,
};
