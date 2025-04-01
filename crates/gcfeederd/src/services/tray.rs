use std::collections::HashMap;
use std::sync::Arc;

use gcfeeder_core::adapter::Port;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::warn;

use super::config;
use crate::config::Config;

const ICON_FILE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resource/icon.png"));

pub struct Service {
    tx_shutdown: oneshot::Sender<oneshot::Sender<()>>,
    rx_quit: mpsc::Receiver<()>,
}

impl Service {
    pub async fn stop(self) {
        let (tx, rx) = oneshot::channel();
        self.tx_shutdown.send(tx).expect("sending shutdown signal");
        rx.await.expect("waiting for shutdown");
    }

    pub async fn recv_quit(&mut self) {
        self.rx_quit.recv().await.expect("waiting for quit");
    }
}

pub fn start(task_tracker: &TaskTracker, config_service: Arc<config::Service>) -> Service {
    let icon = image::load_from_memory(ICON_FILE).expect("load icon");
    let icon_data = icon.into_rgba8();
    let icon_dim = icon_data.dimensions();

    let mut rx_config = config_service.subscribe_config();

    let (tx_shutdown, rx_shutdown) = oneshot::channel();
    let (tx_quit, rx_quit) = mpsc::channel(1);
    let (tx_quit_event_loop, mut rx_quit_event_loop) = oneshot::channel();

    let profile_menu_params = Arc::new(Mutex::new(HashMap::new()));

    let build = move || {
        let tray_menu = tray_icon::menu::Menu::new();
        let _ = tray_menu.append_items(&[
            &tray_icon::menu::MenuItem::with_id("show", "Show", true, None),
            &tray_icon::menu::MenuItem::with_id("hide", "Hide", true, None),
            &tray_icon::menu::MenuItem::with_id("reload", "Reload Config", true, None),
            &tray_icon::menu::PredefinedMenuItem::separator(),
        ]);

        let profile_submenus = Port::all()
            .iter()
            .map(|p| {
                tray_icon::menu::SubmenuBuilder::new()
                    .text(format!("Profile {}", p.index() + 1))
                    .enabled(false)
                    .item(&tray_icon::menu::CheckMenuItem::with_id(
                        "default", "default", false, true, None,
                    ))
                    .build()
                    .expect("build submenu")
            })
            .collect::<Vec<_>>();

        for submenu in &profile_submenus {
            let _ = tray_menu.append(submenu);
        }

        let _ = tray_menu.append_items(&[
            &tray_icon::menu::PredefinedMenuItem::separator(),
            &tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None),
        ]);

        (
            tray_icon::TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("gcfeeder")
                .with_icon(
                    tray_icon::Icon::from_rgba(icon_data.to_vec(), icon_dim.0, icon_dim.1)
                        .expect("icon to be valid"),
                )
                .build()
                .expect("build tray"),
            profile_submenus,
        )
    };

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };

        let profile_menu_params = profile_menu_params.clone();

        task_tracker.spawn_blocking(move || {
            let (_icon, profile_submenus) = build();

            let mut msg = MSG::default();
            while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
                match rx_quit_event_loop.try_recv() {
                    Ok(()) | Err(oneshot::error::TryRecvError::Closed) => break,
                    Err(oneshot::error::TryRecvError::Empty) => {
                        if let Ok(config) = rx_config.try_recv() {
                            let mut profile_menu_params = profile_menu_params.blocking_lock();
                            update_profiles(&config, &profile_submenus, &mut profile_menu_params);
                        }

                        unsafe {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                }
            }
        });
    }

    #[cfg(target_os = "linux")]
    {
        let profile_menu_params = profile_menu_params.clone();

        task_tracker.spawn_blocking(move || {
            let icon = gtk::init().ok().map(|()| build());
            if let Some((_icon, profile_submenus)) = icon {
                loop {
                    match rx_quit_event_loop.try_recv() {
                        Ok(()) | Err(oneshot::error::TryRecvError::Closed) => break,
                        Err(oneshot::error::TryRecvError::Empty) => {
                            if let Ok(config) = rx_config.try_recv() {
                                let mut profile_menu_params = profile_menu_params.blocking_lock();
                                update_profiles(
                                    &config,
                                    &profile_submenus,
                                    &mut profile_menu_params,
                                );
                            }

                            gtk::main_iteration_do(false);
                        }
                    }
                }
            }
        });
    }

    let task_token = Arc::new(CancellationToken::new());

    {
        let task_token = task_token.clone();
        let profile_menu_params = profile_menu_params.clone();
        task_tracker.spawn_blocking(move || {
            run_menu(task_token, tx_quit, config_service, profile_menu_params)
        });
    }

    task_tracker.spawn(run(rx_shutdown, task_token, tx_quit_event_loop));

    Service {
        tx_shutdown,
        rx_quit,
    }
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

        for profile in config.profile.list.keys() {
            let key = format!("port{}_{}", port.index() + 1, profile);
            let checked = config.profile.selected[port.index()] == *profile;
            let item = tray_icon::menu::CheckMenuItem::with_id(&key, profile, true, checked, None);

            let _ = submenu.append(&item);
            profile_menu_params.insert(key, (*port, profile.clone()));
        }

        submenu.set_enabled(true);
    }
}

async fn run(
    rx_shutdown: oneshot::Receiver<oneshot::Sender<()>>,
    task_token: Arc<CancellationToken>,
    tx_quit_event_loop: oneshot::Sender<()>,
) {
    let tx = rx_shutdown.await.expect("recv shutdown");

    task_token.cancel();
    tx_quit_event_loop.send(()).expect("send quit event loop");

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
        unsafe {
            PostQuitMessage(0);
        }
    }

    tx.send(()).expect("send shutdown");
}

fn run_menu(
    token: Arc<CancellationToken>,
    tx_quit: mpsc::Sender<()>,
    config_service: Arc<config::Service>,
    profile_menu_params: Arc<Mutex<HashMap<String, (Port, String)>>>,
) {
    use std::time::Duration;

    use tracing::debug;

    let rx_event = tray_icon::menu::MenuEvent::receiver();

    loop {
        if token.is_cancelled() {
            break;
        }

        // NOTE(Sirius902) A timeout is needed here as the channel is never closed so `recv` could
        // block forever.
        // https://github.com/tauri-apps/tray-icon/issues/244
        if let Ok(event) = rx_event.recv_timeout(Duration::from_millis(100)) {
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
                    let profile_menu_params =
                        { profile_menu_params.blocking_lock().get(id).cloned() };

                    if let Some((port, profile)) = profile_menu_params {
                        debug!("Switching port {:?} profile to \"{}\"", port, profile);

                        config_service.modify_config(Box::new(move |config| {
                            config.profile.selected[port.index()] = profile;
                        }));
                    } else {
                        warn!("Unknown menu event: {id}");
                    }
                }
            }
        }
    }
}
