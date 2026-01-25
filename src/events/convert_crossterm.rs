use crate::events::{CompositeWinitEvent, ConvertEvent, WinitEventState};
use crate::WgpuBackend;

/// Convert winit-events to crossterm-events.
///
/// Unmappable events are dropped.
///
/// __Requires__
///
/// This requires a From<crossterm::event::Event> conversion for your target type.
///
///
/// ```rust ignore
/// let mut convert = ConvertCrossterm::new();
///
/// // ... in window_event()
///
///     if let Some(event) = &event {
///         app.event_type.update_state(
///             event,
///             app.terminal.as_ref().expect("terminal").borrow().backend(),
///         );
///     }
///
/// // ... whenever the window-size changes
///
///     app.event_type.set_window_size(
///         app.window_size,
///         app.terminal.as_ref().expect("terminal").borrow().backend(),
///     );
///
/// // ... convert winit event to your application event type
///
///     let event: MyAppEvent = app.event_type.convert(event);
///
/// // ... or convert to crossterm directly
///
///     let event: crossterm::event::Event = app.event_type.convert(event);
///
/// ```
///
#[derive(Debug, Default)]
pub struct ConvertCrossterm {
    state: WinitEventState,
}

/// Convert winit-events to crossterm-events.
///
/// Any unconvertible events will be sent as a [CompositeWinitEvent]
///
/// __Requires__
///
/// Requires a From<crossterm::event::Event> + From<CompositeWinitEvent> for your
/// target type.
///
#[derive(Debug, Default)]
pub struct ConvertCrosstermEx {
    state: WinitEventState,
}

impl ConvertCrossterm {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<Event> ConvertEvent<Event> for ConvertCrossterm
where
    Event: 'static + From<crossterm::event::Event>,
{
    fn set_window_size(
        &mut self,
        window_size: ratatui_core::backend::WindowSize,
        backend: &WgpuBackend<'_, '_>,
    ) {
        self.state.set_window_size(window_size, backend);
    }

    fn update_state(&mut self, event: &winit::event::WindowEvent, backend: &WgpuBackend<'_, '_>) {
        self.state.update_state(event, backend)
    }

    fn state(&self) -> &WinitEventState {
        &self.state
    }

    fn convert(&mut self, w_event: winit::event::WindowEvent) -> Option<Event> {
        let ct_event = to_crossterm_event(&mut self.state, &w_event);
        if let Some(ct_event) = ct_event {
            Some(ct_event.into())
        } else {
            None
        }
    }
}

impl ConvertCrosstermEx {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<Event> ConvertEvent<Event> for ConvertCrosstermEx
where
    Event: 'static + From<crossterm::event::Event> + From<CompositeWinitEvent>,
{
    fn set_window_size(
        &mut self,
        window_size: ratatui_core::backend::WindowSize,
        backend: &WgpuBackend<'_, '_>,
    ) {
        self.state.set_window_size(window_size, backend);
    }

    fn update_state(&mut self, event: &winit::event::WindowEvent, backend: &WgpuBackend<'_, '_>) {
        self.state.update_state(event, backend)
    }

    fn state(&self) -> &WinitEventState {
        &self.state
    }

    fn convert(&mut self, w_event: winit::event::WindowEvent) -> Option<Event> {
        let ct_event = { to_crossterm_event(&mut self.state, &w_event) };

        if let Some(ct_event) = ct_event {
            Some(ct_event.into())
        } else {
            Some(
                CompositeWinitEvent {
                    event: w_event,
                    state: self.state.clone(),
                }
                    .into(),
            )
        }
    }
}

#[allow(dead_code)]
fn to_crossterm_event(
    state: &mut WinitEventState,
    event: &winit::event::WindowEvent,
) -> Option<crossterm::event::Event> {
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
                            column: state.x,
                            row: state.y,
                            modifiers: ct_key_modifiers,
                        },
                    ))
                } else if state.right_pressed() {
                    Some(crossterm::event::Event::Mouse(
                        crossterm::event::MouseEvent {
                            kind: crossterm::event::MouseEventKind::Drag(
                                crossterm::event::MouseButton::Right,
                            ),
                            column: state.x,
                            row: state.y,
                            modifiers: ct_key_modifiers,
                        },
                    ))
                } else if state.middle_pressed() {
                    Some(crossterm::event::Event::Mouse(
                        crossterm::event::MouseEvent {
                            kind: crossterm::event::MouseEventKind::Drag(
                                crossterm::event::MouseButton::Middle,
                            ),
                            column: state.x,
                            row: state.y,
                            modifiers: ct_key_modifiers,
                        },
                    ))
                } else {
                    Some(crossterm::event::Event::Mouse(
                        crossterm::event::MouseEvent {
                            kind: crossterm::event::MouseEventKind::Moved,
                            column: state.x,
                            row: state.y,
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
                        column: state.x,
                        row: state.y,
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
                        column: state.x,
                        row: state.y,
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
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift)
            } else {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightShift)
            }
        }
        winit::keyboard::NamedKey::Control => {
            if key_location == winit::keyboard::KeyLocation::Left {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftControl)
            } else {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightControl)
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
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftSuper)
            } else {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightSuper)
            }
        }
        winit::keyboard::NamedKey::Meta => {
            if key_location == winit::keyboard::KeyLocation::Left {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftMeta)
            } else {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightMeta)
            }
        }
        winit::keyboard::NamedKey::Hyper => {
            if key_location == winit::keyboard::KeyLocation::Left {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftHyper)
            } else {
                crossterm::event::KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightHyper)
            }
        }
        _ => return None,
    };

    Some(key_code)
}

fn map_mouse_button(button: winit::event::MouseButton) -> Option<crossterm::event::MouseButton> {
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
