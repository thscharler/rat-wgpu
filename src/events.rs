use crate::WgpuBackend;

mod convert_crossterm;
mod convert_winit;

pub use convert_crossterm::{ConvertCrossterm, ConvertCrosstermEx};
pub use convert_winit::ConvertWinit;

///
/// Event-type converter from winit-events to an application event-type.
///
/// Implementors keep track of the modifier state and the mouse position.
///
pub trait ConvertEvent<Event> {
    /// Window size changed.
    fn set_window_size(
        &mut self,
        window_size: ratatui_core::backend::WindowSize,
        backend: &WgpuBackend<'_, '_>,
    );

    /// Update some states.
    fn update_state(&mut self, event: &winit::event::WindowEvent, backend: &WgpuBackend<'_, '_>);

    /// Query the current state.
    fn state(&self) -> &WinitEventState;

    /// Convert winit event.
    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event>;
}

/// Winit event with extra tracked modifier-state and window-size.
#[derive(Debug, Clone)]
pub struct CompositeWinitEvent {
    pub event: winit::event::WindowEvent,
    pub state: WinitEventState,
}

#[derive(Debug, Default, Clone)]
pub struct WinitEventState {
    /// Modifiers.
    modifiers: u16,
    /// Pending dead key
    dead_key_pressed: Option<char>,
    /// Pending dead key
    dead_key_released: Option<char>,
    /// Window size in pixel.
    window_size_px: ratatui_core::layout::Size,
    /// Window size in rendered cells.
    window_size: ratatui_core::layout::Size,
    /// Mouse cursor.
    x: u16,
    /// Mouse cursor.
    y: u16,
    /// Mouse cursor in px
    x_px: f64,
    /// Mouse cursor in px
    y_px: f64,
}

impl WinitEventState {
    const SHIFT: u16 = 0x01;
    const CONTROL: u16 = 0x02;
    const ALT: u16 = 0x04;
    const SUPER: u16 = 0x08;
    const LEFT: u16 = 0x10;
    const MIDDLE: u16 = 0x20;
    const RIGHT: u16 = 0x40;
    const BACK: u16 = 0x80;
    const FORWARD: u16 = 0x100;

    pub fn shift_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::SHIFT) != 0
    }

    pub fn set_shift_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::SHIFT;
        } else {
            self.modifiers &= !WinitEventState::SHIFT;
        }
    }

    pub fn alt_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::ALT) != 0
    }

    pub fn set_alt_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::ALT;
        } else {
            self.modifiers &= !WinitEventState::ALT;
        }
    }

    pub fn ctrl_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::CONTROL) != 0
    }

    pub fn set_ctrl_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::CONTROL;
        } else {
            self.modifiers &= !WinitEventState::CONTROL;
        }
    }

    pub fn super_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::SUPER) != 0
    }

    pub fn set_super_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::SUPER;
        } else {
            self.modifiers &= !WinitEventState::SUPER;
        }
    }

    pub fn window_size(&self) -> ratatui_core::layout::Size {
        self.window_size
    }

    pub fn window_size_px(&self) -> ratatui_core::layout::Size {
        self.window_size_px
    }

    pub fn set_window_size(
        &mut self,
        window_size: ratatui_core::backend::WindowSize,
        backend: &WgpuBackend<'_, '_>,
    ) {
        self.window_size = window_size.columns_rows;
        self.window_size_px = window_size.pixels;

        (self.x, self.y) = backend.pos_to_cell((self.x_px as i32, self.y_px as i32));
    }

    pub fn x(&self) -> u16 {
        self.x
    }

    pub fn set_x(&mut self, x: u16) {
        self.x = x;
    }

    pub fn y(&self) -> u16 {
        self.y
    }

    pub fn set_y(&mut self, y: u16) {
        self.y = y;
    }

    pub fn dead_key_pressed(&self) -> Option<char> {
        self.dead_key_pressed
    }

    pub fn set_dead_key_pressed(&mut self, dc: Option<char>) {
        self.dead_key_pressed = dc;
    }

    pub fn dead_key_released(&self) -> Option<char> {
        self.dead_key_released
    }

    pub fn set_dead_key_released(&mut self, dc: Option<char>) {
        self.dead_key_released = dc;
    }

    pub fn left_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::LEFT) != 0
    }

    pub fn set_left_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::LEFT;
        } else {
            self.modifiers &= !WinitEventState::LEFT;
        }
    }

    pub fn right_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::RIGHT) != 0
    }

    pub fn set_right_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::RIGHT;
        } else {
            self.modifiers &= !WinitEventState::RIGHT;
        }
    }

    pub fn middle_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::MIDDLE) != 0
    }

    pub fn set_middle_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::MIDDLE;
        } else {
            self.modifiers &= !WinitEventState::MIDDLE;
        }
    }

    pub fn back_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::BACK) != 0
    }

    pub fn set_back_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::BACK;
        } else {
            self.modifiers &= !WinitEventState::BACK;
        }
    }

    pub fn forward_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::FORWARD) != 0
    }

    pub fn set_forward_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::FORWARD;
        } else {
            self.modifiers &= !WinitEventState::FORWARD;
        }
    }
}

impl WinitEventState {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn update_state(
        &mut self,
        event: &winit::event::WindowEvent,
        backend: &WgpuBackend<'_, '_>,
    ) {
        match event {
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.set_shift_pressed(modifiers.state().shift_key());
                self.set_alt_pressed(modifiers.state().alt_key());
                self.set_ctrl_pressed(modifiers.state().control_key());
                self.set_super_pressed(modifiers.state().super_key());
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.x_px = position.x;
                self.y_px = position.y;
                (self.x, self.y) = backend.pos_to_cell((self.x_px as i32, self.y_px as i32));
            }
            winit::event::WindowEvent::CursorEntered { .. } => {}
            winit::event::WindowEvent::CursorLeft { .. } => {
                self.x_px = 0.0;
                self.y_px = 0.0;
                self.x = 0;
                self.y = 0;
            }
            winit::event::WindowEvent::MouseWheel { .. } => {}
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let pressed = match state {
                    winit::event::ElementState::Pressed => true,
                    winit::event::ElementState::Released => false,
                };
                match button {
                    winit::event::MouseButton::Left => {
                        self.set_left_pressed(pressed);
                    }
                    winit::event::MouseButton::Right => {
                        self.set_right_pressed(pressed);
                    }
                    winit::event::MouseButton::Middle => {
                        self.set_middle_pressed(pressed);
                    }
                    winit::event::MouseButton::Back => {
                        self.set_back_pressed(pressed);
                    }
                    winit::event::MouseButton::Forward => {
                        self.set_forward_pressed(pressed);
                    }
                    winit::event::MouseButton::Other(_) => {
                        // noop
                    }
                }
            }
            _ => {}
        }
    }
}
