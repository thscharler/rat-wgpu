use crate::WgpuBackend;

pub use crossterm::winit_to_crossterm_event;
pub use winit::event::WindowEvent;

/// Keeps track of button and key states and the current
/// mouse position.
#[derive(Debug, Default, Clone)]
pub struct WinitEventState {
    /// Modifiers.
    pub modifiers: u16,
    /// Pending dead key
    pub dead_key_pressed: Option<char>,
    /// Pending dead key
    pub dead_key_released: Option<char>,
    /// Window size in pixel.
    pub window_size_px: ratatui_core::layout::Size,
    /// Window size in rendered cells.
    pub window_size: ratatui_core::layout::Size,
    /// Mouse cursor in cells (x,y)
    pub mouse: ratatui_core::layout::Position,
    /// Mouse cursor in px (x,y)
    pub mouse_px: (i32, i32),
}

/// Winit event with extra tracked modifier-state and window-size.
#[derive(Debug, Clone)]
pub struct CompositeWinitEvent {
    pub event: winit::event::WindowEvent,
    pub state: WinitEventState,
}

impl WinitEventState {
    pub const SHIFT: u16 = 0x01;
    pub const CONTROL: u16 = 0x02;
    pub const ALT: u16 = 0x04;
    pub const SUPER: u16 = 0x08;
    pub const LEFT: u16 = 0x10;
    pub const MIDDLE: u16 = 0x20;
    pub const RIGHT: u16 = 0x40;
    pub const BACK: u16 = 0x80;
    pub const FORWARD: u16 = 0x100;

    /// Shift key
    pub fn shift_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::SHIFT) != 0
    }

    /// Shift key
    pub fn set_shift_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::SHIFT;
        } else {
            self.modifiers &= !WinitEventState::SHIFT;
        }
    }

    /// Alt key
    pub fn alt_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::ALT) != 0
    }

    /// Alt key
    pub fn set_alt_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::ALT;
        } else {
            self.modifiers &= !WinitEventState::ALT;
        }
    }

    /// Ctrl key
    pub fn ctrl_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::CONTROL) != 0
    }

    /// Ctrl key
    pub fn set_ctrl_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::CONTROL;
        } else {
            self.modifiers &= !WinitEventState::CONTROL;
        }
    }

    /// Super key
    pub fn super_pressed(&self) -> bool {
        (self.modifiers & WinitEventState::SUPER) != 0
    }

    /// Super key
    pub fn set_super_pressed(&mut self, p: bool) {
        if p {
            self.modifiers |= WinitEventState::SUPER;
        } else {
            self.modifiers &= !WinitEventState::SUPER;
        }
    }

    /// Current window size in ratatui cells.
    pub fn window_size(&self) -> ratatui_core::layout::Size {
        self.window_size
    }

    /// Current window size in pixels.
    pub fn window_size_px(&self) -> ratatui_core::layout::Size {
        self.window_size_px
    }

    /// Sets the window-size.
    ///
    /// __Note__
    /// When the window-size changes you should use [update_window_size] instead.
    pub fn set_window_size(&mut self, window_size: ratatui_core::backend::WindowSize) {
        self.window_size = window_size.columns_rows;
        self.window_size_px = window_size.pixels;
    }

    /// Pixel position of the mouse cursor (x,y).
    pub fn mouse_px(&self) -> (i32, i32) {
        self.mouse_px
    }

    /// Pixel position of the mouse cursor (x,y).
    pub fn set_mouse_px(&mut self, pos: (i32, i32)) {
        self.mouse_px = pos;
    }

    /// Cell position of the mouse cursor.
    pub fn mouse(&self) -> ratatui_core::layout::Position {
        self.mouse
    }

    /// Cell position of the mouse cursor.
    pub fn set_mouse(&mut self, pos: ratatui_core::layout::Position) {
        self.mouse = pos;
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

    /// Changes the window-size and recalculates the cell-position.
    pub fn update_window_size(
        &mut self,
        window_size: ratatui_core::backend::WindowSize,
        backend: &WgpuBackend<'_>,
    ) {
        self.window_size = window_size.columns_rows;
        self.window_size_px = window_size.pixels;

        self.mouse = backend.pos_to_cell(self.mouse_px);
    }

    /// Update the state with the event.
    pub fn update_state(&mut self, event: &winit::event::WindowEvent, backend: &WgpuBackend<'_>) {
        match event {
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.set_shift_pressed(modifiers.state().shift_key());
                self.set_alt_pressed(modifiers.state().alt_key());
                self.set_ctrl_pressed(modifiers.state().control_key());
                self.set_super_pressed(modifiers.state().super_key());
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_px = (position.x as i32, position.y as i32);
                self.mouse = backend.pos_to_cell(self.mouse_px);
            }
            winit::event::WindowEvent::CursorEntered { .. } => {}
            winit::event::WindowEvent::CursorLeft { .. } => {
                self.mouse_px = (0, 0);
                self.mouse = ratatui_core::layout::Position::new(0, 0);
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

mod crossterm {
    use crate::events::WinitEventState;

    /// Convert winit-events to crossterm-events.
    /// Unmappable events are returned as None.
    pub fn winit_to_crossterm_event(
        event: (&winit::event::WindowEvent, &mut WinitEventState),
    ) -> Option<crossterm::event::Event> {
        let (event, state) = event;
        'm: {
            match event {
                winit::event::WindowEvent::Resized(_) => {
                    Some(crossterm::event::Event::Resize(
                        state.window_size.width,
                        state.window_size.height,
                    )) //
                }
                winit::event::WindowEvent::Focused(v) => {
                    if *v {
                        Some(crossterm::event::Event::FocusGained)
                    } else {
                        Some(crossterm::event::Event::FocusLost)
                    }
                }
                winit::event::WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            logical_key,
                            location,
                            state: element_state,
                            repeat,
                            ..
                        },
                    ..
                } => {
                    let ct_key_modifiers = map_modifiers(state);
                    let ct_key_event_kind = map_key_state(*element_state, *repeat);
                    let ct_key_event_state = map_key_location(*location);

                    match logical_key {
                        winit::keyboard::Key::Character(c) => {
                            state.dead_key_released = None;
                            state.dead_key_pressed = None;

                            let c = c.as_str().chars().next().expect("char");
                            Some(crossterm::event::Event::Key(
                                crossterm::event::KeyEvent::new_with_kind_and_state(
                                    crossterm::event::KeyCode::Char(c),
                                    ct_key_modifiers,
                                    ct_key_event_kind,
                                    ct_key_event_state,
                                ),
                            ))
                        }
                        winit::keyboard::Key::Named(nk) => {
                            state.dead_key_released = None;
                            state.dead_key_pressed = None;

                            if let Some(kc) = map_key_code(*nk, *location, &state) {
                                Some(crossterm::event::Event::Key(
                                    crossterm::event::KeyEvent::new_with_kind_and_state(
                                        kc,
                                        ct_key_modifiers,
                                        ct_key_event_kind,
                                        ct_key_event_state,
                                    ),
                                ))
                            } else {
                                None
                            }
                        }
                        winit::keyboard::Key::Dead(v) => {
                            if *element_state == winit::event::ElementState::Pressed {
                                track_dead_key(
                                    &mut state.dead_key_pressed,
                                    *v,
                                    ct_key_modifiers,
                                    ct_key_event_kind,
                                    ct_key_event_state,
                                )
                            } else {
                                track_dead_key(
                                    &mut state.dead_key_released,
                                    *v,
                                    ct_key_modifiers,
                                    ct_key_event_kind,
                                    ct_key_event_state,
                                )
                            }
                        }
                        winit::keyboard::Key::Unidentified(_) => None,
                    }
                }
                winit::event::WindowEvent::CursorMoved { .. } => {
                    let ct_key_modifiers = map_modifiers(&state);

                    if state.left_pressed() {
                        Some(crossterm::event::Event::Mouse(
                            crossterm::event::MouseEvent {
                                kind: crossterm::event::MouseEventKind::Drag(
                                    crossterm::event::MouseButton::Left,
                                ),
                                column: state.mouse.x,
                                row: state.mouse.y,
                                modifiers: ct_key_modifiers,
                            },
                        ))
                    } else if state.right_pressed() {
                        Some(crossterm::event::Event::Mouse(
                            crossterm::event::MouseEvent {
                                kind: crossterm::event::MouseEventKind::Drag(
                                    crossterm::event::MouseButton::Right,
                                ),
                                column: state.mouse.x,
                                row: state.mouse.y,
                                modifiers: ct_key_modifiers,
                            },
                        ))
                    } else if state.middle_pressed() {
                        Some(crossterm::event::Event::Mouse(
                            crossterm::event::MouseEvent {
                                kind: crossterm::event::MouseEventKind::Drag(
                                    crossterm::event::MouseButton::Middle,
                                ),
                                column: state.mouse.x,
                                row: state.mouse.y,
                                modifiers: ct_key_modifiers,
                            },
                        ))
                    } else {
                        Some(crossterm::event::Event::Mouse(
                            crossterm::event::MouseEvent {
                                kind: crossterm::event::MouseEventKind::Moved,
                                column: state.mouse.x,
                                row: state.mouse.y,
                                modifiers: ct_key_modifiers,
                            },
                        ))
                    }
                }
                winit::event::WindowEvent::MouseWheel {
                    delta: winit::event::MouseScrollDelta::PixelDelta(_),
                    ..
                } => None,
                winit::event::WindowEvent::MouseWheel {
                    delta: winit::event::MouseScrollDelta::LineDelta(_horizontal, vertical),
                    ..
                } => {
                    let ct_key_modifiers = map_modifiers(&state);

                    Some(crossterm::event::Event::Mouse(
                        crossterm::event::MouseEvent {
                            kind: if *vertical > 0.0 {
                                crossterm::event::MouseEventKind::ScrollUp
                            } else {
                                crossterm::event::MouseEventKind::ScrollDown
                            },
                            column: state.mouse.x,
                            row: state.mouse.y,
                            modifiers: ct_key_modifiers,
                        },
                    ))
                }
                winit::event::WindowEvent::MouseInput {
                    state: mouse_state,
                    button,
                    ..
                } => {
                    let pressed = map_mouse_state(*mouse_state);
                    let Some(ct_button) = map_mouse_button(*button) else {
                        break 'm None;
                    };
                    let ct_key_modifiers = map_modifiers(&state);

                    Some(crossterm::event::Event::Mouse(
                        crossterm::event::MouseEvent {
                            kind: create_mouse_event_kind(ct_button, pressed),
                            column: state.mouse.x,
                            row: state.mouse.y,
                            modifiers: ct_key_modifiers,
                        },
                    ))
                }

                winit::event::WindowEvent::ActivationTokenDone { .. } => None,
                winit::event::WindowEvent::Moved(_) => None,
                winit::event::WindowEvent::CloseRequested => None,
                winit::event::WindowEvent::Destroyed => None,
                winit::event::WindowEvent::DroppedFile(_) => None,
                winit::event::WindowEvent::HoveredFile(_) => None,
                winit::event::WindowEvent::HoveredFileCancelled => None,
                winit::event::WindowEvent::ModifiersChanged(_) => None,
                winit::event::WindowEvent::Ime(_) => None,
                winit::event::WindowEvent::CursorEntered { .. } => None,
                winit::event::WindowEvent::CursorLeft { .. } => None,
                winit::event::WindowEvent::PinchGesture { .. } => None,
                winit::event::WindowEvent::PanGesture { .. } => None,
                winit::event::WindowEvent::DoubleTapGesture { .. } => None,
                winit::event::WindowEvent::RotationGesture { .. } => None,
                winit::event::WindowEvent::TouchpadPressure { .. } => None,
                winit::event::WindowEvent::AxisMotion { .. } => None,
                winit::event::WindowEvent::Touch(_) => None,
                winit::event::WindowEvent::ScaleFactorChanged { .. } => None,
                winit::event::WindowEvent::ThemeChanged(_) => None,
                winit::event::WindowEvent::Occluded(_) => None,
                winit::event::WindowEvent::RedrawRequested => None,
            }
        }
    }

    fn track_dead_key(
        dead_key: &mut Option<char>,
        v: Option<char>,
        ct_key_modifiers: crossterm::event::KeyModifiers,
        ct_key_event_kind: crossterm::event::KeyEventKind,
        ct_key_event_state: crossterm::event::KeyEventState,
    ) -> Option<crossterm::event::Event> {
        if let Some(dk) = *dead_key {
            if let Some(v) = v {
                if v == dk {
                    *dead_key = None;
                    Some(crossterm::event::Event::Key(
                        crossterm::event::KeyEvent::new_with_kind_and_state(
                            crossterm::event::KeyCode::Char(v),
                            ct_key_modifiers,
                            ct_key_event_kind,
                            ct_key_event_state,
                        ),
                    ))
                } else {
                    *dead_key = Some(v);
                    None
                }
            } else {
                unreachable!();
            }
        } else {
            if let Some(v) = v {
                *dead_key = Some(v);
                None
            } else {
                None
            }
        }
    }

    fn map_modifiers(state: &WinitEventState) -> crossterm::event::KeyModifiers {
        let mut m = crossterm::event::KeyModifiers::empty();
        if state.ctrl_pressed() {
            m |= crossterm::event::KeyModifiers::CONTROL;
        }
        if state.shift_pressed() {
            m |= crossterm::event::KeyModifiers::SHIFT;
        }
        if state.alt_pressed() {
            m |= crossterm::event::KeyModifiers::ALT;
        }
        if state.super_pressed() {
            m |= crossterm::event::KeyModifiers::SUPER;
        }
        m
    }

    fn map_key_state(
        state: winit::event::ElementState,
        repeat: bool,
    ) -> crossterm::event::KeyEventKind {
        let mut s = match state {
            winit::event::ElementState::Pressed => crossterm::event::KeyEventKind::Press,
            winit::event::ElementState::Released => crossterm::event::KeyEventKind::Release,
        };
        if repeat {
            s = crossterm::event::KeyEventKind::Repeat;
        }
        s
    }

    fn map_key_location(location: winit::keyboard::KeyLocation) -> crossterm::event::KeyEventState {
        match location {
            winit::keyboard::KeyLocation::Standard => crossterm::event::KeyEventState::NONE,
            winit::keyboard::KeyLocation::Left => crossterm::event::KeyEventState::NONE,
            winit::keyboard::KeyLocation::Right => crossterm::event::KeyEventState::NONE,
            winit::keyboard::KeyLocation::Numpad => crossterm::event::KeyEventState::KEYPAD,
        }
    }

    fn map_key_code(
        named_key: winit::keyboard::NamedKey,
        key_location: winit::keyboard::KeyLocation,
        state: &WinitEventState,
    ) -> Option<crossterm::event::KeyCode> {
        let key_code = match named_key {
            winit::keyboard::NamedKey::Enter => crossterm::event::KeyCode::Enter,
            winit::keyboard::NamedKey::Tab => {
                if state.shift_pressed() {
                    crossterm::event::KeyCode::BackTab
                } else {
                    crossterm::event::KeyCode::Tab
                }
            }
            winit::keyboard::NamedKey::Space => crossterm::event::KeyCode::Char(' '),
            winit::keyboard::NamedKey::ArrowDown => crossterm::event::KeyCode::Down,
            winit::keyboard::NamedKey::ArrowLeft => crossterm::event::KeyCode::Left,
            winit::keyboard::NamedKey::ArrowRight => crossterm::event::KeyCode::Right,
            winit::keyboard::NamedKey::ArrowUp => crossterm::event::KeyCode::Up,
            winit::keyboard::NamedKey::End => crossterm::event::KeyCode::End,
            winit::keyboard::NamedKey::Home => crossterm::event::KeyCode::Home,
            winit::keyboard::NamedKey::PageDown => crossterm::event::KeyCode::PageDown,
            winit::keyboard::NamedKey::PageUp => crossterm::event::KeyCode::PageUp,
            winit::keyboard::NamedKey::Backspace => crossterm::event::KeyCode::Backspace,
            winit::keyboard::NamedKey::Delete => crossterm::event::KeyCode::Delete,
            winit::keyboard::NamedKey::Insert => crossterm::event::KeyCode::Insert,
            winit::keyboard::NamedKey::Escape => crossterm::event::KeyCode::Esc,
            winit::keyboard::NamedKey::F1 => crossterm::event::KeyCode::F(1),
            winit::keyboard::NamedKey::F2 => crossterm::event::KeyCode::F(2),
            winit::keyboard::NamedKey::F3 => crossterm::event::KeyCode::F(3),
            winit::keyboard::NamedKey::F4 => crossterm::event::KeyCode::F(4),
            winit::keyboard::NamedKey::F5 => crossterm::event::KeyCode::F(5),
            winit::keyboard::NamedKey::F6 => crossterm::event::KeyCode::F(6),
            winit::keyboard::NamedKey::F7 => crossterm::event::KeyCode::F(7),
            winit::keyboard::NamedKey::F8 => crossterm::event::KeyCode::F(8),
            winit::keyboard::NamedKey::F9 => crossterm::event::KeyCode::F(9),
            winit::keyboard::NamedKey::F10 => crossterm::event::KeyCode::F(10),
            winit::keyboard::NamedKey::F11 => crossterm::event::KeyCode::F(11),
            winit::keyboard::NamedKey::F12 => crossterm::event::KeyCode::F(12),
            winit::keyboard::NamedKey::F13 => crossterm::event::KeyCode::F(13),
            winit::keyboard::NamedKey::F14 => crossterm::event::KeyCode::F(14),
            winit::keyboard::NamedKey::F15 => crossterm::event::KeyCode::F(15),
            winit::keyboard::NamedKey::F16 => crossterm::event::KeyCode::F(16),
            winit::keyboard::NamedKey::F17 => crossterm::event::KeyCode::F(17),
            winit::keyboard::NamedKey::F18 => crossterm::event::KeyCode::F(18),
            winit::keyboard::NamedKey::F19 => crossterm::event::KeyCode::F(19),
            winit::keyboard::NamedKey::F20 => crossterm::event::KeyCode::F(20),
            winit::keyboard::NamedKey::F21 => crossterm::event::KeyCode::F(21),
            winit::keyboard::NamedKey::F22 => crossterm::event::KeyCode::F(22),
            winit::keyboard::NamedKey::F23 => crossterm::event::KeyCode::F(23),
            winit::keyboard::NamedKey::F24 => crossterm::event::KeyCode::F(24),
            winit::keyboard::NamedKey::F25 => crossterm::event::KeyCode::F(25),
            winit::keyboard::NamedKey::F26 => crossterm::event::KeyCode::F(26),
            winit::keyboard::NamedKey::F27 => crossterm::event::KeyCode::F(27),
            winit::keyboard::NamedKey::F28 => crossterm::event::KeyCode::F(28),
            winit::keyboard::NamedKey::F29 => crossterm::event::KeyCode::F(29),
            winit::keyboard::NamedKey::F30 => crossterm::event::KeyCode::F(30),
            winit::keyboard::NamedKey::F31 => crossterm::event::KeyCode::F(31),
            winit::keyboard::NamedKey::F32 => crossterm::event::KeyCode::F(32),
            winit::keyboard::NamedKey::F33 => crossterm::event::KeyCode::F(33),
            winit::keyboard::NamedKey::F34 => crossterm::event::KeyCode::F(34),
            winit::keyboard::NamedKey::F35 => crossterm::event::KeyCode::F(35),
            winit::keyboard::NamedKey::CapsLock => crossterm::event::KeyCode::CapsLock,
            winit::keyboard::NamedKey::ScrollLock => crossterm::event::KeyCode::ScrollLock,
            winit::keyboard::NamedKey::NumLock => crossterm::event::KeyCode::NumLock,
            winit::keyboard::NamedKey::PrintScreen => crossterm::event::KeyCode::PrintScreen,
            winit::keyboard::NamedKey::Pause => crossterm::event::KeyCode::Pause,
            winit::keyboard::NamedKey::ContextMenu => crossterm::event::KeyCode::Menu,
            winit::keyboard::NamedKey::MediaPlay => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::Play)
            }
            winit::keyboard::NamedKey::MediaPause => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::Pause)
            }
            winit::keyboard::NamedKey::MediaPlayPause => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::PlayPause)
            }
            winit::keyboard::NamedKey::MediaStop => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::Stop)
            }
            winit::keyboard::NamedKey::MediaFastForward => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::FastForward)
            }
            winit::keyboard::NamedKey::MediaRewind => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::Rewind)
            }
            winit::keyboard::NamedKey::MediaTrackNext => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::TrackNext)
            }
            winit::keyboard::NamedKey::MediaTrackPrevious => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::TrackPrevious)
            }
            winit::keyboard::NamedKey::MediaRecord => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::Record)
            }
            winit::keyboard::NamedKey::AudioVolumeDown => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::LowerVolume)
            }
            winit::keyboard::NamedKey::AudioVolumeUp => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::RaiseVolume)
            }
            winit::keyboard::NamedKey::AudioVolumeMute => {
                crossterm::event::KeyCode::Media(crossterm::event::MediaKeyCode::MuteVolume)
            }
            winit::keyboard::NamedKey::Shift => {
                if key_location == winit::keyboard::KeyLocation::Left {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::LeftShift,
                    )
                } else {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::RightShift,
                    )
                }
            }
            winit::keyboard::NamedKey::Control => {
                if key_location == winit::keyboard::KeyLocation::Left {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::LeftControl,
                    )
                } else {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::RightControl,
                    )
                }
            }
            winit::keyboard::NamedKey::Alt => {
                if key_location == winit::keyboard::KeyLocation::Left {
                    crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftAlt)
                } else {
                    crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightAlt)
                }
            }
            winit::keyboard::NamedKey::Super => {
                if key_location == winit::keyboard::KeyLocation::Left {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::LeftSuper,
                    )
                } else {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::RightSuper,
                    )
                }
            }
            winit::keyboard::NamedKey::Meta => {
                if key_location == winit::keyboard::KeyLocation::Left {
                    crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftMeta)
                } else {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::RightMeta,
                    )
                }
            }
            winit::keyboard::NamedKey::Hyper => {
                if key_location == winit::keyboard::KeyLocation::Left {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::LeftHyper,
                    )
                } else {
                    crossterm::event::KeyCode::Modifier(
                        crossterm::event::ModifierKeyCode::RightHyper,
                    )
                }
            }
            _ => return None,
        };

        Some(key_code)
    }

    fn map_mouse_button(
        button: winit::event::MouseButton,
    ) -> Option<crossterm::event::MouseButton> {
        match button {
            winit::event::MouseButton::Left => Some(crossterm::event::MouseButton::Left),
            winit::event::MouseButton::Right => Some(crossterm::event::MouseButton::Right),
            winit::event::MouseButton::Middle => Some(crossterm::event::MouseButton::Middle),
            winit::event::MouseButton::Back => None,
            winit::event::MouseButton::Forward => None,
            winit::event::MouseButton::Other(_) => None,
        }
    }

    fn map_mouse_state(state: winit::event::ElementState) -> bool {
        match state {
            winit::event::ElementState::Pressed => true,
            winit::event::ElementState::Released => false,
        }
    }

    fn create_mouse_event_kind(
        button: crossterm::event::MouseButton,
        pressed: bool,
    ) -> crossterm::event::MouseEventKind {
        match button {
            crossterm::event::MouseButton::Left => {
                if pressed {
                    crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
                } else {
                    crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left)
                }
            }
            crossterm::event::MouseButton::Right => {
                if pressed {
                    crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right)
                } else {
                    crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Right)
                }
            }
            crossterm::event::MouseButton::Middle => {
                if pressed {
                    crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Middle)
                } else {
                    crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Middle)
                }
            }
        }
    }
}
