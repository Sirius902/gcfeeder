use std::env;

use app::App;
use crossbeam::channel;
use egui::Color32;
use gcfeeder_core::adapter::poller::Poller;
use rusb::GlobalContext;

mod app;
pub mod log;
mod util;

const ERROR_COLOR: Color32 = Color32::from_rgb(197, 15, 31);
const WARN_COLOR: Color32 = Color32::from_rgb(193, 156, 0);
const INFO_COLOR: Color32 = Color32::from_rgb(58, 150, 221);
const DEBUG_COLOR: Color32 = Color32::from_rgb(136, 23, 152);

pub fn run() -> eframe::Result<()> {
    let (log_tx, log_rx) = channel::unbounded();
    log::LoggerBuilder::new()
        .sender(log_tx)
        .with_level(::log::LevelFilter::Trace)
        .build()
        .unwrap()
        .init()
        .expect("Failed to set logger");

    const ICON_FILE: &[u8] = include_bytes!("../../resource/icon.png");

    let icon = image::load_from_memory(ICON_FILE).unwrap();
    let icon_data = icon.into_rgba8();
    let icon_dim = icon_data.dimensions();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(600.0, 420.0))
            .with_icon(egui::IconData {
                rgba: icon_data.to_vec(),
                width: icon_dim.0,
                height: icon_dim.1,
            }),
        ..Default::default()
    };

    let _tray_icon = {
        #[allow(unused_variables)]
        let build = move || {
            let tray_menu = tray_icon::menu::Menu::new();
            let _ = tray_menu.append_items(&[
                &tray_icon::menu::MenuItem::with_id("show", "Show", true, None),
                &tray_icon::menu::MenuItem::with_id("hide", "Hide", true, None),
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

            None::<tray_icon::TrayIcon>
        }

        #[cfg(target_os = "linux")]
        {
            std::thread::spawn(move || {
                let icon = gtk::init().ok().and_then(|()| build().ok());
                if icon.is_some() {
                    gtk::main();
                }
            });

            None::<tray_icon::TrayIcon>
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            ::log::warn!("System tray not implemented on this platform");
            None::<tray_icon::TrayIcon>
        }
    };

    let input_source = Poller::new(GlobalContext {});

    eframe::run_native(
        format!("gcfeeder | {}", env!("GCFEEDER_VERSION")).as_str(),
        options,
        Box::new(move |_cc| Ok(Box::new(App::new(input_source, log_rx)))),
    )
}
