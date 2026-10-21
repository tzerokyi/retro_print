# RetroPrint — Rendering Pipeline Upgrade

## Objective
Make the modes visibly different by implementing distinct binary-pixel algorithms, with independent pixel-grid resolution and ink-density controls. Preserve the existing Organic and Fine Stipple renderers.

## Implementation checklist
- [x] Audit the current renderers and inspector integration.
- [x] Add Hard Threshold and Floyd–Steinberg renderers.
- [x] Separate pixel-cell size from grain texture and ink coverage.
- [x] Rework Bayer and Atkinson through the pixel-grid renderer with correct diffusion kernels.
- [x] Preserve source dimensions and original alpha in output.
- [x] Add all engines to the inspector and give existing recipes clear algorithm choices.
- [x] Add tests for determinism, distinct modes, alpha, source dimensions and pixel-cell scaling.
- [x] Verify committed source and available CI status.

## Design contracts
- Organic and Fine Stipple retain their existing implementation and defaults.
- Pixel Size controls grid resolution only for pixel-based engines; Grain Size remains the organic-grain setting.
- Grain Amount controls ordered-dither spread and diffusion intensity; Threshold controls global ink coverage.
- Render the dithering mask on the working grid and reconstruct at the source resolution without smoothing the mask.
- Preserve source alpha per original pixel, including fully transparent pixels.
- UI uses existing RetroPrint naming and colors. No third-party product identifiers or explanatory provenance comments in source code.

## Acceptance criteria
- Threshold, Bayer 4×4, Bayer 8×8, Floyd–Steinberg and Atkinson visibly differ on a gradient.
- Changing Pixel Size changes the apparent pixel blocks without scaling the exported image.
- Preview and export use the same render pipeline, including mode and resolution parameters.
- All modes produce only ink or paper colors, retain alpha semantics, and repeat deterministically.
- Commit code and tests on main; report any build/CI verification limits.

## Verification
- All changes have been committed to main, and file contents were re-read through the GitHub connector.
- Tests were added but could not be executed in the current environment because Rust/Cargo are unavailable.
- GitHub commit-status and workflow-run queries returned no results. Passing CI/build is not asserted.

## Phase 2 — Printable output and editor interaction

- [x] Implement circular halftone dots and diagonal line-screen rendering.
- [x] Add controllable tone-grid blur without modifying legacy Organic/Fine algorithms.
- [x] Use solid inspector scrollbars that allocate their own gutter.
- [x] Enable nearest-neighbor sampling of processed images.
- [x] Export source-resolution vector SVG with merged rectangular runs and retained alpha.
- [x] Add draggable original/result wipe comparison.
- [x] Update mode descriptions, recipes, and README.
- [x] Add SVG merging, opacity, pattern-shape and blur tests as source files.
- [ ] Execute Rust formatting, linting, compilation, unit tests and interactive GUI QA.

## Remaining verification requirements

```bash
cargo fmt --all
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo run --release -- /path/to/sample.png
```

Build/runtime validation is still required before calling this release fully verified.
