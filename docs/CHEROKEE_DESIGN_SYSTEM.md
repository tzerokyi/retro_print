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
- [x] Define reusable radii, matte shadow, action components and container frames in `src/ui.rs`.
- [x] Apply borderless component styling globally with `apply_theme`.
- [x] Migrate toolbar, inspector, swatches, color entry, canvas, and status surface to shared rules.
- [x] Replace instructional labels with affordances, preserve value and semantic labels.
- [x] Conditionally render unavailable actions and empty status/navigation regions.
- [x] Add failure-shake feedback and unit-test the timing.
- [x] Run Format, Clippy, Tests and Linux Xvfb screenshot QA.
- [ ] Confirm the updated native control rendering in a KDE Wayland session.

## Interaction contracts
- The editor remains keyboard accessible and all icon-only actions expose hover/accessibility labels.
- Errors always remain readable; motion is transient and cannot block work.
- Processing modes keep essential parameter labels such as Contrast and Threshold.
- The render engine, settings, seed, export behavior, and source alpha are unchanged.
- Do not add external product names, provenance comments, or placeholder elements.

## Shared controls
- `surface_frame`, `matte_card`: borderless rounded surfaces with matte depth.
- `capsule`, `circle_action`, `icon_button`, `menu_option`: consistent action geometry.
- `choice_popup`: bounded, scrollable option menus; alternatives appear only when opened.
- `slider` / `matte_slider`: precise numeric entry and borderless circular drag control.
- `shake_offset`: brief damped horizontal error feedback.

## Validation
- Empty workspace, image-loaded workspace, selector opening, recipe application, updated preview, scrollbar position, and popup geometry inspected using an X11 virtual display.
- Final source carries no reference-site identifiers and avoids instructional comments.
- Live KDE Wayland integration and optional reduced-motion preference still need native device verification.
