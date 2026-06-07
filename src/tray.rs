#![cfg(target_os = "windows")]

use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct Tray {
    pub icon: TrayIcon,
    pub quit_id: MenuId,
    pub redownload_id: MenuId,
    pub open_config_id: MenuId,
    pub provider_toggle_id: MenuId,
}

pub fn build() -> anyhow::Result<Tray> {
    let menu = Menu::new();
    let status = MenuItem::new("privatewhisper: idle", false, None);
    let provider_toggle = CheckMenuItem::new("Use CUDA provider", true, true, None);
    let redownload = MenuItem::new("Re-download model", true, None);
    let open_config = MenuItem::new("Open config", true, None);
    let quit = MenuItem::new("Quit", true, None);

    menu.append(&status)?;
    menu.append(&provider_toggle)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&redownload)?;
    menu.append(&open_config)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit)?;

    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon_for(crate::app::State::Idle)?)
        .with_tooltip(tooltip_for(crate::app::State::Idle))
        .build()?;

    Ok(Tray {
        icon,
        quit_id: quit.id().clone(),
        redownload_id: redownload.id().clone(),
        open_config_id: open_config.id().clone(),
        provider_toggle_id: provider_toggle.id().clone(),
    })
}

pub fn set_state(tray: &Tray, state: crate::app::State) -> anyhow::Result<()> {
    tray.icon.set_tooltip(Some(tooltip_for(state)))?;
    tray.icon.set_icon(Some(icon_for(state)?))?;
    Ok(())
}

pub fn tooltip_for(state: crate::app::State) -> &'static str {
    match state {
        crate::app::State::Idle => "privatewhisper - idle",
        crate::app::State::Recording => "privatewhisper - recording",
        crate::app::State::Transcribing => "privatewhisper - transcribing",
    }
}

fn icon_for(state: crate::app::State) -> Result<Icon, tray_icon::BadIcon> {
    let color = match state {
        crate::app::State::Idle => [38, 166, 154, 255],
        crate::app::State::Recording => [229, 57, 53, 255],
        crate::app::State::Transcribing => [66, 133, 244, 255],
    };
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for _ in 0..(32 * 32) {
        rgba.extend_from_slice(&color);
    }
    Icon::from_rgba(rgba, 32, 32)
}
