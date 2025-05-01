use std::collections::HashMap;
use std::sync::Arc;

use gcfeeder_core::adapter::Port;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

use super::config;
use crate::config::Config;

const ICON_FILE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resource/icon.png"));

#[derive(Debug)]
pub struct Service {
    tx_shutdown: oneshot::Sender<oneshot::Sender<()>>,
    rx_quit: mpsc::Receiver<()>,
}

impl Service {
    pub async fn stop(self) {
        let (tx, rx) = oneshot::channel();
        self.tx_shutdown.send(tx).expect("sending shutdown signal");

        events::quit();

        rx.await.expect("waiting for shutdown");
    }

    pub async fn recv_quit(&mut self) {
        self.rx_quit.recv().await.expect("waiting for quit");
    }
}

pub fn run(
    rx_config_service: oneshot::Receiver<Arc<config::Service>>,
    tx_service: oneshot::Sender<Service>,
) {
    let config_service = rx_config_service
        .blocking_recv()
        .expect("recv config service");

    // Create config receiver before sending the service to ensure we don't
    // miss config updates.
    let mut rx_config = config_service.subscribe_config();

    let (tx_shutdown, mut rx_shutdown) = oneshot::channel();
    let (tx_quit, rx_quit) = mpsc::channel(1);

    events::setup();

    tx_service
        .send(Service {
            tx_shutdown,
            rx_quit,
        })
        .expect("send service");

    let icon = image::load_from_memory(ICON_FILE).expect("load icon");
    let icon_data = icon.into_rgba8();
    let icon_dim = icon_data.dimensions();

    let tray_menu = tray_icon::menu::Menu::new();
    tray_menu
        .append_items(&[
            &tray_icon::menu::MenuItem::with_id("show", "Show", true, None),
            &tray_icon::menu::MenuItem::with_id("hide", "Hide", true, None),
            &tray_icon::menu::MenuItem::with_id("reload", "Reload Config", true, None),
            &tray_icon::menu::PredefinedMenuItem::separator(),
        ])
        .expect("tray append");

    for port in Port::all() {
        tray_menu
            .append(&make_empty_profile_menu(*port))
            .expect("tray append");
    }

    tray_menu
        .append_items(&[
            &tray_icon::menu::PredefinedMenuItem::separator(),
            &tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None),
        ])
        .expect("tray append");

    let icon = tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu.clone()))
        .with_tooltip("gcfeeder")
        .with_icon(
            tray_icon::Icon::from_rgba(icon_data.to_vec(), icon_dim.0, icon_dim.1)
                .expect("icon to be valid"),
        )
        .build()
        .expect("build tray");

    // Use icon as template on macOS.
    icon.set_icon_as_template(true);

    let rx_menu_event = tray_icon::menu::MenuEvent::receiver();
    let mut profile_params = HashMap::new();

    let handle_messages = move || {
        match rx_shutdown.try_recv() {
            Ok(tx) => {
                tx.send(()).expect("send shutdown signal");
                return true;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                return true;
            }
            Err(oneshot::error::TryRecvError::Empty) => {}
        }

        if let Ok(config) = rx_config.try_recv() {
            update_profiles(&config, &tray_menu, &mut profile_params);
        }

        if let Ok(event) = rx_menu_event.try_recv() {
            match event.id.as_ref() {
                "show" => {
                    // TODO(Sirius902) Implement.
                    debug!("Show");
                }
                "hide" => {
                    // TODO(Sirius902) Implement.
                    debug!("Hide");
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
                    if let Some((port, profile)) = profile_params.get(id).cloned() {
                        debug!("Switching port {:?} profile to \"{}\"", port, profile);

                        config_service.modify_config(Box::new(move |config| {
                            config.profile.selected[port.index()] = profile;
                        }));

                        if let Ok(config) = rx_config.blocking_recv() {
                            update_profiles(&config, &tray_menu, &mut profile_params);
                        }
                    } else {
                        warn!("Unknown menu event: {id}");
                    }
                }
            }
        }

        false
    };

    events::run(handle_messages);
}

fn profile_menu_id(port: Port) -> tray_icon::menu::MenuId {
    format!("profile{}", port.index() + 1).into()
}

fn make_empty_profile_menu(port: Port) -> tray_icon::menu::Submenu {
    tray_icon::menu::SubmenuBuilder::new()
        .id(profile_menu_id(port))
        .text(format!("Profile {}", port.index() + 1))
        .enabled(false)
        .build()
        .expect("build submenu")
}

// FUTURE(Sirius902) Only rebuild necessary parts. We have to be careful to avoid KDE caching
// issues though.
fn update_profiles(
    config: &Config,
    menu: &tray_icon::menu::Menu,
    profile_menu_params: &mut HashMap<String, (Port, String)>,
) {
    profile_menu_params.clear();

    for port in Port::all() {
        let id = profile_menu_id(*port);
        let index = menu
            .items()
            .iter()
            .position(|item| *item.id() == id)
            .expect("profile submenu is in tray");

        // Remove the old submenu and build the new one.
        assert_eq!(*menu.remove_at(index).expect("removing submenu").id(), id);

        let submenu = make_empty_profile_menu(*port);

        let mut add_profile = |profile: &str| {
            let key = format!("port{}_{}", port.index() + 1, profile);
            let checked = config.profile.selected[port.index()] == *profile;
            let item = tray_icon::menu::CheckMenuItem::with_id(&key, profile, true, checked, None);

            submenu.append(&item).expect("tray append");
            profile_menu_params.insert(key, (*port, profile.to_string()));
        };

        let mut keys: Vec<_> = config
            .profile
            .list
            .keys()
            .filter(|k| *k != "default")
            .collect();

        keys.sort();

        add_profile("default");
        submenu
            .append(&tray_icon::menu::PredefinedMenuItem::separator())
            .expect("tray append");

        for profile in keys {
            add_profile(profile);
        }

        submenu.set_enabled(true);

        menu.insert(&submenu, index).expect("tray insert");
    }
}

#[cfg(target_os = "windows")]
mod events {
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG, WM_NULL,
    };

    static EVENT_THREAD_ID: AtomicU32 = AtomicU32::new(0);

    pub fn setup() {
        EVENT_THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
    }

    pub fn quit() {
        let event_thread_id = EVENT_THREAD_ID.load(Ordering::Relaxed);
        assert!(event_thread_id != 0, "event_thread_id is set");

        let _ = unsafe { PostThreadMessageW(event_thread_id, WM_NULL, WPARAM(0), LPARAM(0)) };
    }

    pub fn run(mut handle_messages: impl FnMut() -> bool + 'static) {
        let mut msg = MSG::default();
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if handle_messages() {
                break;
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod events {
    use tracing::warn;

    pub fn setup() {
        if let Err(err) = gtk::init() {
            warn!("Failed to init gtk: {err}");
        }
    }

    pub fn quit() {
        // Unblock `gtk::main_iteration`.
        glib::idle_add(|| glib::ControlFlow::Continue);
    }

    pub fn run(mut handle_messages: impl FnMut() -> bool + 'static) {
        loop {
            gtk::main_iteration();

            if handle_messages() {
                break;
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod events {
    use dispatch::Queue;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEventMask};
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};

    pub fn setup() {
        let mtm = MainThreadMarker::new().expect("on main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    }

    pub fn quit() {
        Queue::main().exec_async(|| {
            let mtm = MainThreadMarker::new().expect("on main thread");
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.terminate(None) };
        });
    }

    pub fn run(mut handle_messages: impl FnMut() -> bool + 'static) {
        let mtm = MainThreadMarker::new().expect("on main thread");
        let app = NSApplication::sharedApplication(mtm);
        let run_loop = unsafe { NSRunLoop::currentRunLoop() };

        loop {
            let event = unsafe {
                app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    NSEventMask::Any,
                    None,
                    NSDefaultRunLoopMode,
                    true,
                )
            };

            if let Some(event) = event {
                unsafe {
                    app.sendEvent(&event);
                }
            }

            if handle_messages() {
                break;
            }

            unsafe { run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &NSDate::distantFuture()) };
        }
    }
}
