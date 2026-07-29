#!/usr/bin/env python3
"""Probe the Rust scene ABI against the hand-written WGPU shader layout.

This is intentionally small and dependency-free so it can be run from a dirty
working tree while investigating Linux rendering failures:

    python3 docs/tools/wgpu_shader_layout_probe.py

The current WGPU shader declares a shorter Background/Quad than Rust sends to
the GPU. That makes storage-buffer instance indexing drift after the first quad.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
COLOR_RS = ROOT / "crates/gpui/src/color.rs"
SCENE_RS = ROOT / "crates/gpui/src/scene.rs"
WGSL = ROOT / "crates/gpui-wgpu/src/shaders.wgsl"


@dataclass(frozen=True)
class Layout:
    name: str
    rust_size: int
    rust_fields: tuple[str, ...]
    wgsl_expected_fields: tuple[str, ...]
    current_size: int


def contains_all(source: str, snippets: tuple[str, ...]) -> bool:
    return all(snippet in source for snippet in snippets)


def main() -> int:
    color_rs = COLOR_RS.read_text()
    scene_rs = SCENE_RS.read_text()
    wgsl = WGSL.read_text()

    layouts = (
        Layout(
            name="Background",
            rust_size=128,
            rust_fields=(
                "pub(crate) colors: [LinearColorStop; 4]",
                "pub(crate) stop_count: u32",
                "pub(crate) center: [f32; 2]",
                "pub(crate) radius: [f32; 2]",
            ),
            wgsl_expected_fields=(
                "colors: array<LinearColorStop, 4>",
                "stop_count: u32",
                "center: vec2<f32>",
                "radius: vec2<f32>",
            ),
            current_size=72,
        ),
        Layout(
            name="Quad",
            rust_size=256,
            rust_fields=(
                "pub continuous_corners: u32",
                "pub transform: TransformationMatrix",
                "pub blend_mode: u32",
            ),
            wgsl_expected_fields=(
                "continuous_corners: u32",
                "transform: TransformationMatrix",
                "blend_mode: u32",
            ),
            current_size=160,
        ),
    )

    print("WGPU shader layout probe")
    print(f"repo: {ROOT}")
    print()

    failed = False
    for layout in layouts:
        rust_source = color_rs if layout.name == "Background" else scene_rs
        rust_has_fields = contains_all(rust_source, layout.rust_fields)
        wgsl_has_expected_fields = contains_all(wgsl, layout.wgsl_expected_fields)
        wgsl_size = layout.rust_size if wgsl_has_expected_fields else layout.current_size
        size_matches = layout.rust_size == wgsl_size

        print(f"{layout.name}:")
        print(f"  rust expected size: {layout.rust_size} bytes")
        print(f"  wgsl current size:  {wgsl_size} bytes")
        print(f"  rust fields found:  {rust_has_fields}")
        print(f"  wgsl fields found:  {wgsl_has_expected_fields}")

        if not size_matches or not wgsl_has_expected_fields:
            failed = True
            print("  verdict: MISMATCH")
        else:
            print("  verdict: ok")
        print()

    if failed:
        print("Conclusion: Rust sends larger scene structs than WGPU reads.")
        print("Fix WGSL Background/Quad first, then re-test alpha/frame hypotheses.")
        return 1

    print("Conclusion: no known WGPU scene ABI mismatch detected.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
