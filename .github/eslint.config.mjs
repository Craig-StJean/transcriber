// ESLint flat config for the GNOME Shell extension (GJS, ES modules).
// Lives under .github/ so it isn't copied into the installed extension.
// Run: npx eslint@9 --config .github/eslint.config.mjs extension/
import js from "@eslint/js";

export default [
    js.configs.recommended,
    {
        files: ["extension/**/*.js"],
        languageOptions: {
            ecmaVersion: 2022,
            sourceType: "module",
            globals: {
                // GJS / GNOME Shell runtime globals
                global: "readonly",
                log: "readonly",
                logError: "readonly",
                print: "readonly",
                printerr: "readonly",
                console: "readonly",
                imports: "readonly",
                TextDecoder: "readonly",
                TextEncoder: "readonly",
                setTimeout: "readonly",
                clearTimeout: "readonly",
                setInterval: "readonly",
                clearInterval: "readonly",
            },
        },
        rules: {
            "no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
        },
    },
];
