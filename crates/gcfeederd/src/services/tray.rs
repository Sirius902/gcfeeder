use std::sync::Arc;

use gcfeeder_core::adapter::Port;
use tokio::sync::mpsc;
use tracing::warn;

use super::config;

const ICON_FILE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resource/icon.png"));

pub struct Service {
    rx_quit: mpsc::Receiver<()>,
}

impl Service {
    pub async fn recv_quit(&mut self) {
        self.rx_quit.recv().await.expect("waiting for quit");
    }
}

pub fn start(config_service: Arc<config::Service>) -> Service {
    let icon = image::load_from_memory(ICON_FILE).expect("load icon");
    let icon_data = icon.into_rgba8();
    let icon_dim = icon_data.dimensions();

    let (tx_quit, rx_quit) = mpsc::channel(1);

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let build = move || {
        let tray_menu = tray_icon::menu::Menu::new();
        let _ = tray_menu.append_items(&[
            &tray_icon::menu::MenuItem::with_id("show", "Show", true, None),
            &tray_icon::menu::MenuItem::with_id("hide", "Hide", true, None),
            &tray_icon::menu::MenuItem::with_id("reload", "Reload Config", true, None),
            &tray_icon::menu::PredefinedMenuItem::separator(),
        ]);

        // TODO(Sirius902) Rebuild tray when config updates.
        // TODO(Sirius902) Implement switching profiles from these button.
        for i in 0..Port::COUNT {
            let _ = tray_menu.append(
                &tray_icon::menu::SubmenuBuilder::new()
                    .id(format!("profile{i}").into())
                    .text(format!("Profile {}", i + 1))
                    .item(&tray_icon::menu::MenuItem::with_id(
                        "default", "default", true, None,
                    ))
                    .build()
                    .expect("build submenu"),
            );
        }

        let _ = tray_menu.append_items(&[
            &tray_icon::menu::PredefinedMenuItem::separator(),
            &tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None),
        ]);

        tray_icon::TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("gcfeeder")
            .with_icon(
                tray_icon::Icon::from_rgba(icon_data.to_vec(), icon_dim.0, icon_dim.1)
                    .expect("icon to be valid"),
            )
            .build()
    };

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };

        std::thread::spawn(move || {
            let icon = build().ok();
            if icon.is_some() {
                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        });
    }

    #[cfg(target_os = "linux")]
    {
        std::thread::spawn(move || {
            let icon = gtk::init().ok().and_then(|()| build().ok());
            if icon.is_some() {
                gtk::main();
            }
        });
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        tracing::warn!("System tray not implemented on this platform");
    }

    std::thread::spawn(move || run_menu(tx_quit, config_service.clone()));

    Service { rx_quit }
}

fn run_menu(tx_quit: mpsc::Sender<()>, config_service: Arc<config::Service>) {
    let rx_event = tray_icon::menu::MenuEvent::receiver();

    while let Ok(event) = tokio::task::block_in_place(|| rx_event.recv()) {
        match event.id.as_ref() {
            "show" => {
                // TODO(Sirius902) Implement.
            }
            "hide" => {
                // TODO(Sirius902) Implement.
            }
            "reload" => {
                config_service.reload_config();
            }
            "quit" => {
                if let Err(err) = tx_quit.try_send(()) {
                    warn!("Failed to send quit: {err}");
                }
            }
            id => {
                warn!("Unknown menu event: {id}");
            }
        }
    }
}
