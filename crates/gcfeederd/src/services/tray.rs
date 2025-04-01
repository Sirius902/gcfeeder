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
    let _ = tray_menu.append_items(&[
        &tray_icon::menu::MenuItem::with_id("show", "Show", true, None),
        &tray_icon::menu::MenuItem::with_id("hide", "Hide", true, None),
        &tray_icon::menu::MenuItem::with_id("reload", "Reload Config", true, None),
        &tray_icon::menu::PredefinedMenuItem::separator(),
    ]);

    let profile_submenus: [_; Port::COUNT] = std::array::from_fn(|i| {
        tray_icon::menu::SubmenuBuilder::new()
            .text(format!("Profile {}", i + 1))
            .enabled(false)
            .build()
            .expect("build submenu")
    });

    for submenu in &profile_submenus {
        let _ = tray_menu.append(submenu);
    }

    let _ = tray_menu.append_items(&[
        &tray_icon::menu::PredefinedMenuItem::separator(),
        &tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None),
    ]);

    let _icon = tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("gcfeeder")
        .with_icon(
            tray_icon::Icon::from_rgba(icon_data.to_vec(), icon_dim.0, icon_dim.1)
                .expect("icon to be valid"),
        )
        .build()
        .expect("build tray");

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
            update_profiles(&config, &profile_submenus, &mut profile_params);
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
                            update_profiles(&config, &profile_submenus, &mut profile_params);
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

fn update_profiles(
    config: &Config,
    profile_submenus: &[tray_icon::menu::Submenu],
    profile_menu_params: &mut HashMap<String, (Port, String)>,
) {
    profile_menu_params.clear();

    for port in Port::all() {
        let submenu = &profile_submenus[port.index()];

        submenu.set_enabled(false);

        while submenu.remove_at(0).is_some() {}

        let mut add_profile = |profile: &str| {
            let key = format!("port{}_{}", port.index() + 1, profile);
            let checked = config.profile.selected[port.index()] == *profile;
            let item = tray_icon::menu::CheckMenuItem::with_id(&key, profile, true, checked, None);

            let _ = submenu.append(&item);
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
        let _ = submenu.append(&tray_icon::menu::PredefinedMenuItem::separator());

        for profile in keys {
            add_profile(profile);
        }

        submenu.set_enabled(true);
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

// TODO(Sirius902) Call `HANDLE_MESSAGES` after each application event is received.
#[cfg(target_os = "macos")]
mod events {
    use std::cell::RefCell;

    use dispatch::Queue;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    thread_local! {
        static HANDLE_MESSAGES: RefCell<Option<Box<dyn FnMut() -> bool>>> = const { RefCell::new(None) };
    }

    pub fn setup() {
        let mtm = MainThreadMarker::new().expect("on main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    }

    pub fn quit() {
        Queue::main().exec_async(|| {
            // TODO(Sirius902) Do this in NSApplicationDelegate event handler instead.
            HANDLE_MESSAGES.with_borrow_mut(|f| f.as_mut().expect("exists")());

            let mtm = MainThreadMarker::new().expect("on main thread");
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.terminate(None) };
        });
    }

    pub fn run(handle_messages: impl FnMut() -> bool + 'static) {
        HANDLE_MESSAGES.with_borrow_mut(|f| {
            *f = Some(Box::new(handle_messages));
        });

        let mtm = MainThreadMarker::new().expect("on main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.run();
    }
}
