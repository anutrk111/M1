# ADR 0041 — Normalized BoundingBox Canon

## Status

Accepted. Supersedes the pixel-geometry clause (item 2) of [ADR-0011](../../../Module%205/docs/adr/0011-multi-plate-detection-ids.md).

## Context

The M01 skeleton used `x,y,w,h: f32` with no stated unit; the M05 PDR used `u32` pixels. Talos ingests images of mixed resolution and calls multiple AI providers that report geometry differently. A pixel canon ties every stored box to one frame size and forces every consumer to carry dimensions.

## Decision

1. The canonical `talos_types::BoundingBox` is normalized `f32` in `[0.0, 1.0]` relative to the **original** M02 frame:

   ```rust
   pub struct BoundingBox { pub x_min: f32, pub y_min: f32, pub x_max: f32, pub y_max: f32 }
   ```

2. Invariants: finite values, each within `[0, 1]`, `x_min < x_max`, `y_min < y_max`. `BoundingBox::new` and serde deserialization both enforce them. The legacy `{x,y,w,h}` shape is rejected.
3. Pixel `u32` geometry (`PixelRect { x, y, w, h }`) is **derived only** at the crop/render boundary via `BoundingBox::to_pixel_rect(frame_width, frame_height)` and is never persisted as the canonical box. The conversion expands outward to whole pixels, snaps values within 1e-3 px of a whole pixel, clamps to the frame, and yields at least 1x1.
4. `BoundingBox::from_pixels` normalizes provider pixel boxes. M03 normalizes provider geometry, and M05 validates it, before a `Detection` exists.

## Consequences

- Contracts stay resolution-independent across providers and image sizes.
- M04 Stage 2 needs the original frame dimensions to crop (they are available from decode).
- The decision/export schema moves to `2.1` (`DECISION_SCHEMA_VERSION`).
