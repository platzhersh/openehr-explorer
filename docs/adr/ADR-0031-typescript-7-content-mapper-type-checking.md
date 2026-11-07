# ADR-0031: TypeScript 7.1 and the Vue Content Mapper Replace vue-tsc

**Date:** 2026-11-07
**Status:** Accepted
**Deciders:** Development Team
**Related:** ADR-0007 (Pinned Dependency Versions)

## Context

The frontend type-check was `vue-tsc --noEmit`. `vue-tsc` works by patching the JavaScript TypeScript compiler, so it cannot run on TypeScript 7, whose compiler is a native Go rewrite with no JavaScript API to patch. Moving to TypeScript 7.0 was therefore blocked, and `typescript` had to stay on 6.x (`vue-tsc` 3.x fails with `ERR_PACKAGE_PATH_NOT_EXPORTED` on 7.0).

TypeScript 7.1 (beta) adds an official plugin point called **content mappers**: a package registered in `tsconfig.json` that turns a non-TypeScript file (here `.vue`) into virtual TypeScript plus a source map, so `tsc` itself reports errors at the right positions in the original file. Vue Language Tools v4 (alpha) ships `@vue/content-mapper` on top of it, built on the same `@vue/language-core` that `vue-tsc` uses.

## Decision

1. **Type-check with `tsc --runExternalCode`** (TypeScript `7.1.0-beta`) and `@vue/content-mapper` (`4.0.0-alpha.3`). `vue-tsc` is removed. `npm run build` runs `tsc --runExternalCode && vite build`.
2. **Register the mapper in `tsconfig.json`** under `contentMappers` (a sibling of `compilerOptions`, not inside it). Vue compiler options go in the mapper's `options`, not a top-level `vueCompilerOptions`, which v4 ignores silently. `--runExternalCode` is required because a content mapper is an npm package that `tsc` executes.
3. **Keep `vue-tsc` 3 behaviour with two mapper options:** `fallthroughAttributes: true` and `dataAttributes: ["data-*"]`. Without them v4 reports 46 errors on valid code (mostly `data-*` attributes).
4. **Declare custom elements in `src/global-components.d.ts`.** v4 always checks unknown components, whereas `vue-tsc` 3 did not. `vite.config.mjs` treats every `mb-*` tag as a custom element at runtime, so each `mb-*` tag used in a template (currently `mb-auto-form`) needs a `GlobalComponents` entry.
5. **Pin `typescript` with an npm `overrides` entry.** `7.1.0-beta` is a prerelease and does not satisfy the `typescript >= 4.9.x` peer range of Storybook's packages, so installs fail without it. Remove the override once TypeScript 7.1 is stable.
6. **Versions stay pinned** per ADR-0007, including the beta and alpha.

## Consequences

- TypeScript 7 is usable in this repo. A full type-check takes about 2 seconds locally.
- The checks are equivalent: a deliberately broken template is still caught, with the same error codes as `vue-tsc`, because both use `@vue/language-core`.
- The compiler is a beta and the mapper is an alpha. Expect churn until 7.1 and Vue Language Tools v4 are stable, and bump both together.
- **Editor support is not ready.** The Vue language server still needs classic TypeScript, so features such as go-to-definition into `.vue` files and rename in templates may regress. CI uses TypeScript 7.1 while editors may use an older TypeScript; a mismatch between the two can show different errors.
- v4's stricter rules apply to new code: attributes passed to a component must be declared as props (apart from `data-*`), and global directives and `mb-*` elements need type declarations. This is the accepted cost of the stricter checking.
- Known mapper issues to watch: an empty event handler such as `@click.stop=""` produces a syntax error that stops type-checking, so write `@click.stop`; and `allowUnreachableCode: false` can report false "unreachable code" errors in templates.
- If the mapper proves too unstable, the fallback is `typescript` 6.0.x with `vue-tsc` 3 (the state of `main` before this change).

## Alternatives Considered

- **Stay on TypeScript 6.0.3 with `vue-tsc` 3.** Works today and is stable, but TypeScript 7 stays blocked and the project depends on a compiler-patching approach.
- **Alias `typescript` to `@typescript/typescript6` and run `vue-tsc` on it.** Keeps type-checking working, but the project is not on TypeScript 7 in practice and gains nothing.
- **TypeScript 7.0 with plain `tsc`.** Cannot read `.vue` files, so it errors on every component import.

## References

- [Goodbye vue-tsc? Moving a Vue Project to TypeScript 7.1 Content Mappers](https://alexop.dev/posts/goodbye-vue-tsc-typescript-7-content-mappers/)
