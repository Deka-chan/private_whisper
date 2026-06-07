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

const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 72.0;
const BOTTOM_OFFSET: i32 = 90;
const BAR_COUNT: usize = 24;
const BACKGROUND: u32 = 0x0011_1114; // near-black pill
const TRACK: u32 = 0x002E_2E36; // dim resting color
const RECORDING_ACCENT: u32 = 0x00EA_EAF2; // soft white
const TRANSCRIBING_ACCENT: u32 = 0x006A_A0FF; // soft blue

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
    bars: Vec<f32>,
    mode: Mode,
    frame: u64,
}

impl Overlay {
    pub fn new<T>(target: &EventLoopWindowTarget<T>, level: Arc<AtomicU32>) -> anyhow::Result<Self> {
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
            bars: vec![0.0; BAR_COUNT],
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

    /// Advance one animation frame: turn the mic level (or a synthetic pulse) into
    /// a lively per-bar target and ease each bar toward it.
    pub fn tick(&mut self) {
        self.frame = self.frame.saturating_add(1);
        let f = self.frame as f32;

        // Overall amplitude in [0,1].
        let amp = match self.mode {
            Mode::Idle => 0.0,
            Mode::Recording => {
                // Mic peaks for speech are small; boost + soft curve so quiet
                // speech is clearly visible and loud speech reaches the top.
                let peak = f32::from_bits(self.level.load(Ordering::Relaxed)).clamp(0.0, 1.0);
                (peak * 6.5).powf(0.7).clamp(0.0, 1.0)
            }
            Mode::Transcribing => 0.34 + 0.12 * (f * 0.16).sin(),
        };

        let center = (BAR_COUNT - 1) as f32 / 2.0;
        for (i, bar) in self.bars.iter_mut().enumerate() {
            // Bell-ish envelope: taller in the middle, shorter at the edges.
            let dist = (i as f32 - center).abs() / center.max(1.0);
            let shape = 1.0 - 0.5 * dist;
            // Per-bar oscillation so neighbouring bars move independently.
            let osc = 0.5 + 0.5 * (f * 0.35 + i as f32 * 1.9).sin();
            let target = (amp * shape * osc).clamp(0.0, 1.0);
            // Ease toward the target for fluid motion (no jumps).
            *bar += (target - *bar) * 0.4;
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

        let accent = match self.mode {
            Mode::Idle => TRACK,
            Mode::Recording => RECORDING_ACCENT,
            Mode::Transcribing => TRANSCRIBING_ACCENT,
        };

        let mut buffer = self
            .surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;
        buffer.fill(BACKGROUND);
        draw_bars(&mut buffer, width_px, height_px, &self.bars, accent);
        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        Ok(())
    }
}

/// Draw thin, vertically-mirrored bars. Each bar's height and brightness scale
/// with its magnitude: a dim short stub at rest, a tall bright bar when loud.
fn draw_bars(buffer: &mut [u32], width: usize, height: usize, bars: &[f32], accent: u32) {
    if width == 0 || height == 0 || bars.is_empty() {
        return;
    }
    let n = bars.len();
    let pitch = width as f32 / n as f32;
    let bar_w = (pitch * 0.42).round().clamp(2.0, 7.0) as usize;
    let cy = height / 2;
    let max_h = height.saturating_sub(12).max(4);
    let min_h = bar_w.max(3); // resting stub ~ a dot

    for (i, &mag) in bars.iter().enumerate() {
        let mag = mag.clamp(0.0, 1.0);
        let cx = ((i as f32 + 0.5) * pitch).round() as isize;
        let x0 = (cx - bar_w as isize / 2).max(0) as usize;
        let x1 = (x0 + bar_w).min(width);
        let h = min_h + ((max_h.saturating_sub(min_h)) as f32 * mag).round() as usize;
        let color = lerp_color(TRACK, accent, mag.powf(0.7));

        let half = h / 2;
        let y0 = cy.saturating_sub(half);
        let y1 = (cy + half + (h & 1)).min(height);
        for y in y0..y1 {
            let row = y * width;
            for x in x0..x1 {
                buffer[row + x] = color;
            }
        }
    }
}

fn lerp_color(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u32, y: u32| -> u32 {
        let x = x as f32;
        let y = y as f32;
        (x + (y - x) * t).round() as u32
    };
    let r = lerp((a >> 16) & 0xff, (b >> 16) & 0xff);
    let g = lerp((a >> 8) & 0xff, (b >> 8) & 0xff);
    let bl = lerp(a & 0xff, b & 0xff);
    (r << 16) | (g << 8) | bl
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
