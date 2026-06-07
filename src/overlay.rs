#![cfg(target_os = "windows")]

use std::{num::NonZeroU32, rc::Rc};

use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event_loop::EventLoopWindowTarget,
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Window, WindowBuilder, WindowId},
};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_HIDE, SW_SHOWNOACTIVATE,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    },
};

const WIDTH: f64 = 260.0;
const HEIGHT: f64 = 44.0;
const BOTTOM_OFFSET: i32 = 80;
const RED: u32 = 0x00E5_3935;
const BLUE: u32 = 0x0042_85F4;

pub struct Overlay {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    color: u32,
    visible: bool,
}

impl Overlay {
    pub fn new<T>(target: &EventLoopWindowTarget<T>) -> anyhow::Result<Self> {
        let window = Rc::new(
            WindowBuilder::new()
                .with_title("privatewhisper-overlay")
                .with_decorations(false)
                .with_resizable(false)
                .with_always_on_top(true)
                .with_visible(false)
                .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
                .with_skip_taskbar(true)
                .build(target)?,
        );

        set_click_through_no_activate(hwnd(&window));
        position_bottom_center(target, &window);

        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        Ok(Self {
            window,
            surface,
            color: RED,
            visible: false,
        })
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn set_state(&mut self, state: crate::app::State) {
        let hwnd = hwnd(&self.window);

        match state {
            crate::app::State::Idle => {
                self.visible = false;
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
            }
            crate::app::State::Recording => {
                self.color = RED;
                self.visible = true;
                unsafe {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                self.window.request_redraw();
            }
            crate::app::State::Transcribing => {
                self.color = BLUE;
                self.visible = true;
                unsafe {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                self.window.request_redraw();
            }
        }
    }

    pub fn redraw(&mut self) {
        if !self.visible {
            return;
        }

        if let Err(err) = self.try_redraw() {
            log::warn!("failed to redraw overlay: {err:#}");
        }
    }

    fn try_redraw(&mut self) -> anyhow::Result<()> {
        let size = self.window.inner_size();
        let width = NonZeroU32::new(size.width.max(1)).expect("width clamped to non-zero");
        let height = NonZeroU32::new(size.height.max(1)).expect("height clamped to non-zero");

        self.surface
            .resize(width, height)
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        let mut buffer = self
            .surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;
        buffer.fill(self.color);
        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        Ok(())
    }
}

fn hwnd(window: &Window) -> HWND {
    window.hwnd() as HWND
}

fn set_click_through_no_activate(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = style | (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
    }
}

fn position_bottom_center<T>(target: &EventLoopWindowTarget<T>, window: &Window) {
    let Some(monitor) = target.primary_monitor() else {
        return;
    };

    let monitor_size = monitor.size();
    let monitor_position = monitor.position();
    let window_size = window.outer_size();

    let x = monitor_position.x + (monitor_size.width as i32 - window_size.width as i32) / 2;
    let y =
        monitor_position.y + monitor_size.height as i32 - window_size.height as i32 - BOTTOM_OFFSET;
    window.set_outer_position(PhysicalPosition::new(x, y));
}
