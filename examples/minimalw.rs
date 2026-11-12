use crossterm::event::Event;
use rat_theme4::palette::Colors;
use rat_theme4::theme::SalsaTheme;
use rat_theme4::{StyleName, WidgetStyle, create_salsa_theme};
use rat_wgpu::events::{WindowEventState, winit_to_crossterm_event};
use rat_wgpu::font::{fallback_emoji_font, fallback_font};
use rat_wgpu::wgpu::Backends;
use rat_wgpu::{Builder, WgpuBackend};
use rat_widget::event::{Dialog, HandleEvent, MenuOutcome, Outcome, Regular, ct_event, event_flow};
use rat_widget::focus::{FocusBuilder, impl_has_focus};
use rat_widget::menu::{MenuLine, MenuLineState};
use rat_widget::msgdialog::{MsgDialog, MsgDialogState};
use rat_widget::statusline_stacked::StatusLineStacked;
use ratatui_core::backend::Backend;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::style::Style;
use ratatui_core::terminal::Terminal;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

// application for winit.
// split because of Terminal::draw()
pub struct MinimalWinit {
    ctx: Option<MinimalContext>,
    state: MinimalState,
}

#[allow(dead_code)]
pub struct MinimalContext {
    window: Arc<Window>,
    terminal: Terminal<WgpuBackend<'static>>,
    event_state: WindowEventState,
    font_size: u32,
}

// Actual application state.
pub struct MinimalState {
    theme: SalsaTheme,
    menu: MenuLineState,
    error_dlg: MsgDialogState,
    status: String,

    count: usize,
    last_render: Duration,
    last_event: Duration,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::builder().build()?;

    let theme = create_salsa_theme("Imperial Shell");

    let mut app = MinimalWinit {
        ctx: None,
        state: MinimalState {
            theme,
            menu: Default::default(),
            error_dlg: Default::default(),
            status: Default::default(),
            count: Default::default(),
            last_render: Default::default(),
            last_event: Default::default(),
        },
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}

impl ApplicationHandler for MinimalWinit {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // window
        let mut attr = WindowAttributes::default();
        attr.visible = false;
        let window = Arc::new(event_loop.create_window(attr).unwrap());

        let size = window.inner_size();

        // async because wgpu
        let terminal = Terminal::new(
            futures_lite::future::block_on(
                Builder::new()
                    .with_fonts([
                        fallback_font().expect("fallback"),
                        fallback_emoji_font().expect("emoji"),
                    ])
                    .with_backends(Backends::from_comma_list("vulkan"))
                    .with_width_and_height(size.width, size.height)
                    .build_with_target(window.clone()),
            )
            .unwrap(),
        )
        .unwrap();

        window.set_visible(true);
        window.request_redraw();

        self.ctx = Some(MinimalContext {
            window,
            terminal,
            event_state: Default::default(),
            font_size: 20,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }

        let Some(ctx) = self.ctx.as_mut() else { return };

        let mut rr = Outcome::Continue;

        if let WindowEvent::Resized(size) = event {
            ctx.terminal.backend_mut().resize(size.width, size.height);
            let size = ctx
                .terminal
                .backend_mut()
                .window_size()
                .expect("window-size");
            _ = ctx.terminal.resize(Rect::from(size.columns_rows));
            ctx.event_state
                .update_window_size(size, ctx.terminal.backend());
        }
        if let WindowEvent::RedrawRequested = event {
            rr = Outcome::Changed;
        }
        ctx.event_state.update_state(&event, ctx.terminal.backend());

        if rr == Outcome::Continue {
            if let Ok(ctevent) = winit_to_crossterm_event((&event, &mut ctx.event_state)) {
                let ttt = SystemTime::now();
                let r = process_event(&ctevent, &mut self.state, ctx);
                self.state.last_event = ttt.elapsed().expect("duration");
                match r {
                    Ok(r) => {
                        rr = r;
                    }
                    Err(_) => {
                        event_loop.exit();
                        return;
                    }
                }
            }
        }

        if rr == Outcome::Changed {
            let ttt = SystemTime::now();
            let frame = ctx
                .terminal
                .draw(|frame| {
                    let area = frame.area();
                    let buf = frame.buffer_mut();
                    render_tui(area, buf, &mut self.state);
                })
                .expect("render");
            self.state.count = frame.count;
            self.state.last_render = ttt.elapsed().expect("duration");
        }
    }
}

impl_has_focus!(menu for MinimalState);

fn render_tui(area: Rect, buf: &mut Buffer, state: &mut MinimalState) {
    let layout = Layout::vertical([
        Constraint::Fill(1), //
        Constraint::Length(1),
    ])
    .split(area);

    let status_layout = Layout::horizontal([
        Constraint::Fill(61), //
        Constraint::Fill(39),
    ])
    .split(layout[1]);

    MenuLine::new()
        .styles(state.theme.style(WidgetStyle::MENU))
        .title("-!-")
        .item_parsed("_Quit")
        .render(status_layout[0], buf, &mut state.menu);

    if state.error_dlg.active() {
        MsgDialog::new()
            .styles(state.theme.style(WidgetStyle::MSG_DIALOG))
            .render(layout[0], buf, &mut state.error_dlg);
    }

    // Status
    let status_color_1 = state.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 3);
    let status_color_2 = state.theme.p.fg_bg_style(Colors::White, 0, Colors::Blue, 2);

    StatusLineStacked::new()
        .style(state.theme.style(Style::STATUS_BASE))
        .center_margin(1)
        .center(Line::from(state.status.as_str()))
        .end(
            Span::from(format!(
                " R({:03}){:05} ",
                state.count,
                format!("{:.0?}", state.last_render)
            ))
            .style(status_color_1),
            Span::from(" "),
        )
        .end_bare(
            Span::from(format!(" E{:05} ", format!("{:.0?}", state.last_event)))
                .style(status_color_2),
        )
        .render(status_layout[1], buf);
}

fn process_event(
    event: &Event,
    state: &mut MinimalState,
    ctx: &mut MinimalContext,
) -> Result<Outcome, ()> {
    match event {
        ct_event!(resized) => return Ok(Outcome::Changed),
        ct_event!(key press CONTROL-'q') => return Err(()),
        ct_event!(scroll CONTROL down for _x,_y) => {
            let bk = ctx.terminal.backend_mut();

            ctx.font_size += 1;

            let size = bk.update_font_size(ctx.font_size);
            _ = ctx.terminal.resize(Rect::from(size.columns_rows));
            ctx.event_state
                .update_window_size(size, ctx.terminal.backend());
            return Ok(Outcome::Changed);
        }
        ct_event!(scroll CONTROL up for _x,_y) => {
            let bk = ctx.terminal.backend_mut();

            ctx.font_size -= 1;

            let size = bk.update_font_size(ctx.font_size);
            _ = ctx.terminal.resize(Rect::from(size.columns_rows));
            ctx.event_state
                .update_window_size(size, ctx.terminal.backend());
            return Ok(Outcome::Changed);
        }
        _ => {}
    }

    event_flow!({
        if state.error_dlg.active() {
            state.error_dlg.handle(event, Dialog).into()
        } else {
            Outcome::Continue
        }
    });

    let mut focus = FocusBuilder::build_for(state);
    let f = focus.handle(event, Regular);

    event_flow!(match state.menu.handle(event, Regular) {
        MenuOutcome::Activated(0) => return Err(()),
        v => Outcome::from(v),
    });

    Ok(f)
}
