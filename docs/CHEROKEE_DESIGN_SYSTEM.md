# Cherokee Design System — RetroPrint

## Principles
1. **Conditional presence.** A control or container appears only when its content or action is relevant.
2. **Borderless depth.** Use fill, spacing, and soft matte shadows instead of outlined surfaces or separators.
3. **Continuous corners.** Containers are rounded rectangles.
4. **Action geometry.** Click, toggle, drag, and selection controls use capsules or circles.
5. **Content geometry.** Static content and preview surfaces use rounded rectangles.
6. **Failure feedback.** Rejected actions retain an accessible error message and apply a brief horizontal shake; never use motion as the sole signal.
7. **No instructional captions.** Avoid persistent explanation text. Keep necessary control names, real values, and accessibility/hover names; these communicate identity rather than instructions.

## Shared implementation
- [ ] Define reusable radii, matte shadow, action components and container frames in `src/ui.rs`.
- [ ] Apply borderless component styling globally with `apply_theme`.
- [ ] Migrate toolbar, inspector, swatches, color entry, canvas, and status surface to shared rules.
- [ ] Replace instructional labels with affordances, preserve value and semantic labels.
- [ ] Conditionally render unavailable actions and empty status/navigation regions.
- [ ] Add failure-shake feedback and unit-test the timing.
- [ ] Run Format, Clippy, Tests, desktop builds, and interactive screenshot QA.

## Interaction contracts
- The editor remains keyboard accessible and all icon-only actions expose hover/accessibility labels.
- Errors always remain readable; motion is transient and cannot block work.
- Processing modes keep essential parameter labels such as Contrast and Threshold.
- The render engine, settings, seed, export behavior, and source alpha are unchanged.
- Do not add external product names, provenance comments, or placeholder elements.
