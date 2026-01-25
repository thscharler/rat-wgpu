use chrono::Local;
use rat_wgpu::font::FontData;
use rat_wgpu::{Builder, WgpuBackend};
use ratatui_core::style::Stylize;
use ratatui_core::terminal::Terminal;
use ratatui_core::text::Line;
use ratatui_widgets::block::Block;
use ratatui_widgets::paragraph::Paragraph;
use std::sync::Arc;
use wgpu::Backends;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;
use winit::window::WindowAttributes;

pub struct App<'d> {
    window: Option<Arc<Window>>,
    backend: Option<Terminal<WgpuBackend<'d, 'static>>>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::builder().build()?;

    let mut app = App {
        window: None,
        backend: None,
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut attr = WindowAttributes::default();
        attr.visible = false;
        self.window = Some(Arc::new(event_loop.create_window(attr).unwrap()));

        let size = self.window.as_ref().unwrap().inner_size();

        self.backend = Some(
            Terminal::new(
                futures_lite::future::block_on(
                    Builder::new()
                        .with_fonts([
                            FontData.fallback_font().expect("fallback"),
                            FontData.fallback_emoji_font().expect("emoji"),
                        ])
                        .with_backends(Backends::from_comma_list("vulkan"))
                        .with_width_and_height(size.width, size.height)
                        .build_with_target(self.window.as_ref().unwrap().clone()),
                )
                .unwrap(),
            )
            .unwrap(),
        );

        self.window.as_ref().unwrap().set_visible(true);
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }

        let Some(terminal) = self.backend.as_mut() else {
            return;
        };

        if let WindowEvent::Resized(size) = event {
            terminal.backend_mut().resize(size.width, size.height);
        }

        terminal
            .draw(|f| {
                f.render_widget(
                    Paragraph::new(Line::from(vec![
                        "Hello World! 🦀🚀".bold().italic(),
                        format!(" It is {}", Local::now().format("%H:%M:%S.%f")).dim(),
                    ]))
                    .block(Block::bordered()),
                    f.area(),
                );
            })
            .unwrap();

        self.window.as_ref().unwrap().request_redraw();
    }
}
