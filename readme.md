![semver](https://img.shields.io/badge/semver-☑-FFD700)
![stable](https://img.shields.io/badge/stability-stable-8A2BE2)
[![crates.io](https://img.shields.io/crates/v/rat-wgpu.svg)](https://crates.io/crates/rat-wgpu)
[![Documentation](https://docs.rs/rat-salsa-wgpu/badge.svg)](https://docs.rs/rat-wgpu)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/license-APACHE-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
![](https://tokei.rs/b1/github/thscharler/rat-wgpu)

# rat-wgpu

A wgpu based rendering backend for ratatui.

Kudos to https://github.com/Jesterhearts/ratatui-wgpu
which was the starting point for this crate. I reuse
a lot of the actual wgpu parts and the details of the
font-rendering.

__Improvements__

* Fixes quite a few bugs with glyph rendering and uses
  a different approach for placing the glyphs on screen.
* Can map screen coordinates accurately back to
  cell coordinates even with bidi text.
* Cursor and text blinking.
* Image rendering.
* Easier font management.
* Support to map winit events to crossterm events.
* Easier switch of the postprocessing stage.

__Running__

Start with examples/minimalw.rs.

It starts from bare bones and sets up a minimal winit
event loop.