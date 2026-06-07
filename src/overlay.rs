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
const HEIGHT: f64 = 54.0;
const BOTTOM_OFFSET: i32 = 96;
const BAR_COUNT: usize = 46;
const BACKGROUND: u32 = 0x0012_1216; // near-black pill
const TRACK: u32 = 0x002C_2C34; // dim baseline color
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
    /// Captured amplitude history: index 0 is oldest (left), last is newest (right).
    /// Each value is a frozen moment that scrolls left over time — a little slice
    /// of the recorded waveform, not a live "in the moment" equalizer.
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
                // Reset the trace so the next recording starts from a clean line.
                for v in &mut self.history {
                    *v = 0.0;
                }
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

    /// Capture the current amplitude as one new sample on the right and scroll the
    /// whole history left — a moving slice of the recorded waveform.
    pub fn tick(&mut self) {
        self.frame = self.frame.saturating_add(1);
        let f = self.frame as f32;

        let sample = match self.mode {
            Mode::Idle => 0.0,
            Mode::Recording => {
                // Mic peaks for speech are small; boost + soft curve so quiet
                // speech is clearly visible and loud speech reaches the top.
                let peak = f32::from_bits(self.level.load(Ordering::Relaxed)).clamp(0.0, 1.0);
                (peak * 6.5).powf(0.7).clamp(0.0, 1.0)
            }
            // No live audio while transcribing — scroll a gentle synthetic wave so
            // the trace keeps moving and reads as "working".
            Mode::Transcribing => (0.30 + 0.16 * (f * 0.45).sin()).clamp(0.0, 1.0),
        };

        if !self.history.is_empty() {
            self.history.rotate_left(1);
            if let Some(last) = self.history.last_mut() {
                *last = sample;
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
        draw_history(&mut buffer, width_px, height_px, &self.history, accent);
        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer: {e}"))?;

        Ok(())
    }
}

/// Draw the captured history as thin, vertically-mirrored bars. Older samples
/// (left) fade out so the trace looks like it is scrolling away.
fn draw_history(buffer: &mut [u32], width: usize, height: usize, history: &[f32], accent: u32) {
    if width == 0 || height == 0 || history.is_empty() {
        return;
    }
    let n = history.len();
    let pitch = width as f32 / n as f32;
    let bar_w = (pitch * 0.55).round().clamp(2.0, 4.0) as usize;
    let cy = height / 2;
    let max_h = height.saturating_sub(8).max(4);
    let min_h = 2usize;
    let denom = (n - 1).max(1) as f32;

    for (i, &mag) in history.iter().enumerate() {
        let mag = mag.clamp(0.0, 1.0);
        let cx = ((i as f32 + 0.5) * pitch).round() as isize;
        let x0 = (cx - bar_w as isize / 2).max(0) as usize;
        let x1 = (x0 + bar_w).min(width);

        let h = min_h + ((max_h.saturating_sub(min_h)) as f32 * mag).round() as usize;
        // Brightness from amplitude, then a left-to-right fade for the scroll feel.
        let fade = 0.32 + 0.68 * (i as f32 / denom);
        let color = scale_color(lerp_color(TRACK, accent, mag.powf(0.7)), fade);

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

fn scale_color(c: u32, factor: f32) -> u32 {
    let factor = factor.clamp(0.0, 1.0);
    let scale = |x: u32| -> u32 { ((x as f32) * factor).round() as u32 };
    let r = scale((c >> 16) & 0xff);
    let g = scale((c >> 8) & 0xff);
    let bl = scale(c & 0xff);
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
