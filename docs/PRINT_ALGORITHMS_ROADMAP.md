# RetroPrint — Advanced Print Algorithms: Plan and Verification

## Objective
Implement independent rendering logic for missing classical error diffusion, blue-noise ordered dithering, clustered-dot printing, and directional crosshatching; maintain compatibility with existing renderers and integrate all algorithms with UI, presets, comparison gallery and lossless exports.

## References
- Classical JJN / Stucki / Sierra convolution weights and total error: https://numbersandshapes.net/posts/error_diffusion/
- Jarvis and Stucki error filter design contracts: https://eclipse-imagen.github.io/imagen/guide/image-manipulation/
- Digital halftoning, void-and-cluster and clustered-dot formulations: https://research.ibm.com/publications/a-unified-framework-for-digital-halftoning-and-dither-mask-construction-variations-on-a-theme-and-implementation-issues
- Christoph Peters CC0 blue-noise threshold mask: `assets/noise/blue-noise-128.png`, `assets/noise/LICENSE.txt`.

## Phase 1 — Engine
- [x] Jarvis–Judice–Ninke: normalized 48-weight three-row diffusion, mirrored serpentine scan.
- [x] Stucki: normalized 42-weight three-row diffusion, mirrored serpentine scan.
- [x] Sierra: normalized 32-weight three-row diffusion, mirrored serpentine scan.
- [x] Blue Noise: independent threshold map backed by the licensed bundled 128×128 mask.
- [x] Clustered Dots: rotated analytic cluster-dot growth and highlight reversal.
- [x] Crosshatch: four directional line layers controlled by source tone.
- [x] Preserve all nine existing methods and preview-to-export pipeline.

## Phase 2 — UI and Recipes
- [x] Register fifteen algorithms in the texture mode inspector.
- [x] Add one distinct recipe per algorithm, fifteen total.
- [x] Show settings controls only where they affect the algorithm.
- [x] Keep grain/organic controls decoupled from pixel grid controls.

## Phase 3 — Export and QA
- [x] Maintain exact-size, alpha-preserving indexed PNG output.
- [x] Compress SVG with same-color path batching and merged rectangular runs.
- [x] Expand comparison gallery from 3×3 to 3×5; produce per-method coverage CSV.
- [x] Add repeatability, geometry, tonal ordering and alpha regression tests.
- [x] Confirm cargo formatting, Clippy, 37 unit tests and 15-mode gallery in GitHub Actions ([passing run](https://github.com/tzerokyi/retro_print/actions/runs/39695505855)).
- [x] Linux portable ZIP, Windows portable ZIP and Flatpak bundle build successfully ([CI run](https://github.com/tzerokyi/retro_print/actions/runs/39695505855)).
- [x] Linux binary opens under Xvfb (1600×1000), loads a sample PNG and reports Preview ready.
- [ ] Compare against proprietary BDFM reference output using identical source photos; matching output requires those reference images.

## Reproduce
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo run --locked --example algorithm_gallery -- target/effect-gallery
cargo run --locked --example algorithm_gallery -- target/effect-gallery /path/to/reference.png
```

## Acceptance
All fifteen methods have distinct processing logic; presets no longer reduce to nine shared engines. JJN/Stucki/Sierra weights match cited literature. Blue Noise, Clustered Dots and Crosshatch use separate threshold/screen logic. Original transparency and source dimensions remain intact. Do not describe the output as a verified proprietary BDFM clone.

## Verified gallery
- Synthetic comparison input: 480 × 320 with gradient, geometry, circles and checkerboard.
- Engine images: 15; recipe images: 15; contact sheets: 2; coverage reports: 2.
- Total assertions: 37 passing (0 failed).
- Actual comparison sheets reviewed; distinct screening, diffusion, dots and hatching patterns visible.
- This does not prove an exact reproduction of unavailable proprietary BDFM output.
