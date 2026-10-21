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
- GitHub Actions runs cargo fmt, strict clippy, unit tests, and visual gallery generation.
- Nine engine modes and nine distinct presets generate 18 individual PNGs and two contact sheets.
- Automated tests, formatting, and linting passed at commit 7ac6ae9.
- Native interactive desktop verification remains separate from automated CI.

## Phase 2 — Printable output and editor interaction

- [x] Implement circular halftone dots and diagonal line-screen rendering.
- [x] Add controllable tone-grid blur without modifying legacy Organic/Fine algorithms.
- [x] Use solid inspector scrollbars that allocate their own gutter.
- [x] Enable nearest-neighbor sampling of processed images.
- [x] Export source-resolution vector SVG with merged rectangular runs and retained alpha.
- [x] Add draggable original/result wipe comparison.
- [x] Update mode descriptions, recipes, and README.
- [x] Add SVG merging, opacity, pattern-shape and blur tests as source files.
- [x] Execute Rust formatting, linting, compilation, and unit tests in CI.
- [ ] Complete manual interactive GUI QA on KDE and Windows.

## Remaining verification requirements

```bash
cargo fmt --all
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo run --release -- /path/to/sample.png
```

Build/runtime validation is still required before calling this release fully verified.

## Phase 3 — Visual QA and canvas navigation

- [x] Add one-to-one engine recipes for all nine modes.
- [x] Improve the halftone spot function to cover highlights and shadows smoothly.
- [x] Add a monotonic halftone response regression test.
- [x] Add synchronized canvas zoom and pan, plus Fit reset.
- [x] Clamp the viewport image while dragging and validate the geometry in tests.
- [x] Add a deterministic built-in synthetic test source with gradients, silhouette, sharp edges, and fine texture.
- [x] Add the `algorithm_gallery` executable example for optional real-photo comparisons.
- [x] Create nine engine outputs, nine styled recipe outputs, two contact sheets, and a position manifest.
- [x] Upload generated comparison images as a GitHub Actions artifact.
- [x] Inspect the generated synthetic comparison sheets and confirm all nine outputs are visually distinct.
- [ ] Verify zoom/pan controls and scrollbar layout on an interactive KDE Plasma session.
- [ ] Inspect SVG results in a vector editor and test output from a real photographic image.

### Reproduce the visual comparison

```bash
cargo run --locked --example algorithm_gallery -- target/effect-gallery
cargo run --locked --example algorithm_gallery -- target/effect-gallery /path/to/photo.png
```

The `retroprint-effect-gallery` artifact is generated in the validation job.
