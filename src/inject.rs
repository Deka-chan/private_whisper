/// Normalize raw ASR output before insertion. Currently trims edges; kept as a
/// seam for future punctuation/casing tweaks.
pub fn normalize(raw: &str) -> String {
    raw.trim().to_string()
}

#[cfg(target_os = "windows")]
use std::{thread, time::Duration};

#[cfg(target_os = "windows")]
use arboard::Clipboard;

#[cfg(target_os = "windows")]
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Superwhisper-style insertion: save the current clipboard, set our text,
/// send Ctrl+V, wait `delay_ms`, then restore the previous clipboard.
///
/// On paste failure after the clipboard is set, the text is left on the
/// clipboard so the caller can notify the user and manual paste still works.
#[cfg(target_os = "windows")]
pub fn paste(text: &str, delay_ms: u64) -> anyhow::Result<()> {
    let text = normalize(text);
    if text.is_empty() {
        return Ok(());
    }

    let mut clip = Clipboard::new()?;
    let previous = clip.get_text().ok();

    clip.set_text(text)?;

    // Use the V *virtual-key code* (VK_V = 0x56), not Key::Unicode('v').
    // Key::Unicode translates the char through the ACTIVE keyboard layout via
    // VkKeyScanW; with a non-Latin layout active (e.g. Russian) 'v' maps to -1
    // and enigo errors ("key state could not be converted to u32"). Key::Other
    // is sent as a raw VIRTUAL_KEY, so Ctrl+V works regardless of layout.
    const VK_V: u32 = 0x56;
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(Key::Control, Direction::Press)?;
    enigo.key(Key::Other(VK_V), Direction::Click)?;
    enigo.key(Key::Control, Direction::Release)?;

    thread::sleep(Duration::from_millis(delay_ms));

    if let Some(prev) = previous {
        let _ = clip.set_text(prev);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize("  привет мир \n"), "привет мир");
    }

    #[test]
    fn empty_or_whitespace_becomes_empty() {
        assert_eq!(normalize("   \n\t"), "");
    }
}
