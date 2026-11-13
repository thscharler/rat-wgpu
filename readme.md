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
