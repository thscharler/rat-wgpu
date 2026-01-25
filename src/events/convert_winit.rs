use crate::WgpuBackend;
use crate::events::{CompositeWinitEvent, ConvertEvent, WinitEventState};

/// Converts a bare winit::event::WindowEvent to a [CompositeWinitEvent]
/// that keeps track of the modifier state and the cursor position.
#[derive(Debug, Default)]
pub struct ConvertWinit {
    state: WinitEventState,
}

impl<Event> ConvertEvent<Event> for ConvertWinit
where
    Event: 'static + From<CompositeWinitEvent>,
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

    fn convert(&mut self, event: winit::event::WindowEvent) -> Option<Event> {
        Some(
            CompositeWinitEvent {
                event,
                state: self.state.clone(),
            }
            .into(),
        )
    }
}

impl ConvertWinit {
    pub fn new() -> Self {
        Self::default()
    }
}
