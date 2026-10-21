# RetroPrint — Rendering Pipeline Upgrade

## Objective
Make the modes visibly different by implementing distinct binary-pixel algorithms, with independent pixel-grid resolution and ink-density controls. Preserve the existing Organic and Fine Stipple renderers.

## Implementation checklist
- [x] Audit the current renderers and inspector integration.
- [ ] Add Hard Threshold and Floyd–Steinberg renderers.
- [ ] Separate pixel-cell size from grain texture and ink coverage.
- [ ] Rework Bayer and Atkinson through the pixel-grid renderer with correct diffusion kernels.
- [ ] Preserve source dimensions and original alpha in output.
- [ ] Add all engines to the inspector and give existing recipes clear algorithm choices.
- [ ] Add tests for determinism, distinct modes, alpha, source dimensions and pixel-cell scaling.
- [ ] Verify committed source and available CI status.

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
