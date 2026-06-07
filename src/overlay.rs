#![cfg(target_os = "windows")]

use std::{
    num::NonZeroU32,
    rc::Rc,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event_loop::EventLoopWindowTarget,
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Window, WindowBuilder, WindowId},
};
use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn},
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_HIDE, SW_SHOWNOACTIVATE,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    },
};

const WIDTH: f64 = 300.0;
const HEIGHT: f64 = 64.0;
const BOTTOM_OFFSET: i32 = 80;
const BAR_COUNT: usize = 18;
const BACKGROUND: u32 = 0x001C_1C1E;
const RECORDING_BAR: u32 = 0x0080_E0C0;
const TRANSCRIBING_BAR: u32 = 0x006A_A0FF;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Idle,
    Recording,
    Transcribing,
}

pub struct Overlay {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    level: Arc<AtomicU32>,
    history: Vec<f32>,
    mode: Mode,
    frame: u64,
}

impl Overlay {
    pub fn new<T>(
        target: &EventLoopWindowTarget<T>,
        level: Arc<AtomicU32>,
    ) -> anyhow::Result<Self> {
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
        apply_pill_region(hwnd(&window), &window);
        position_bottom_center(target, &window);

        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        Ok(Self {
            window,
            surface,
            level,
            history: vec![0.0; BAR_COUNT],
            mode: Mode::Idle,
            frame: 0,
        })
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn is_active(&self) -> bool {
        self.mode != Mode::Idle
    }

    pub fn set_state(&mut self, state: crate::app::State) {
        let hwnd = hwnd(&self.window);

        match state {
            crate::app::State::Idle => {
                self.mode = Mode::Idle;
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
            }
            crate::app::State::Recording => {
                self.mode = Mode::Recording;
                unsafe {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                self.window.request_redraw();
            }
            crate::app::State::Transcribing => {
                self.mode = Mode::Transcribing;
                unsafe {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                self.window.request_redraw();
            }
        }
    }

    pub fn tick(&mut self) {
        self.frame = self.frame.saturating_add(1);
        let v = match self.mode {
            Mode::Idle => 0.0,
            Mode::Recording => {
                let level = self.level.load(Ordering::Relaxed);
                f32::from_bits(level).clamp(0.0, 1.0)
            }
            Mode::Transcribing => 0.25 + 0.2 * ((self.frame as f32 * 0.25).sin() * 0.5 + 0.5),
        };

        if !self.history.is_empty() {
            self.history.rotate_left(1);
            if let Some(last) = self.history.last_mut() {
                *last = v;
            }
            let trailing_len = self.history.len().saturating_sub(1);
            for value in &mut self.history[..trailing_len] {
                *value *= 0.92;
            }
        }

        self.window.request_redraw();
    }

    pub fn redraw(&mut self) {
        if !self.is_active() {
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
        let width_px = width.get() as usize;
        let height_px = height.get() as usize;

        self.surface
            .resize(width, height)
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        let mut buffer = self
            .surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;
        buffer.fill(BACKGROUND);

        let bar_color = match self.mode {
            Mode::Idle => BACKGROUND,
            Mode::Recording => RECORDING_BAR,
            Mode::Transcribing => TRANSCRIBING_BAR,
        };

        draw_bars(&mut buffer, width_px, height_px, &self.history, bar_color);

        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        Ok(())
    }
}

fn draw_bars(buffer: &mut [u32], width: usize, height: usize, history: &[f32], color: u32) {
    if width == 0 || height == 0 || history.is_empty() {
        return;
    }

    let bar_count = history.len();
    let pitch = width as f32 / bar_count as f32;
    let bar_width = (width as f32 / (bar_count as f32 * 2.0)).max(2.0) as usize;
    let max_height = height.saturating_sub(6).max(3);

    for (i, &mag) in history.iter().enumerate() {
        let center_x = ((i as f32 + 0.5) * pitch).round() as isize;
        let x0 = (center_x - bar_width as isize / 2).max(0) as usize;
        let x1 = (x0 + bar_width).min(width);
        let bar_height = ((0.12 + 0.80 * mag.clamp(0.0, 1.0)) * height as f32).round() as usize;
        let bar_height = bar_height.clamp(3, max_height);
        let y0 = height.saturating_sub(bar_height) / 2;
        let y1 = (y0 + bar_height).min(height);

        for y in y0..y1 {
            let row = y * width;
            for x in x0..x1 {
                buffer[row + x] = color;
            }
        }
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

fn apply_pill_region(hwnd: HWND, window: &Window) {
    let size = window.outer_size();
    let width = size.width as i32;
    let height = size.height as i32;
    unsafe {
        let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, height, height);
        if !region.is_null() {
            SetWindowRgn(hwnd, region, 1);
        }
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
