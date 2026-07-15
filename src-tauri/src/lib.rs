mod sleep;

use sleep::{ChangeOutcome, SleepStatus};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

const SYNC_INTERVAL: Duration = Duration::from_secs(3);

struct AppState {
    operation_lock: Arc<Mutex<()>>,
}

async fn read_in_blocking_task() -> Result<SleepStatus, String> {
    tauri::async_runtime::spawn_blocking(sleep::read_sleep_status)
        .await
        .map_err(|error| format!("状態取得workerが失敗しました: {error}"))?
}

async fn set_in_blocking_task(disabled: bool) -> Result<ChangeOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || sleep::set_sleep_disabled(disabled))
        .await
        .map_err(|error| format!("設定変更workerが失敗しました: {error}"))?
}

#[tauri::command]
async fn get_sleep_state(state: tauri::State<'_, AppState>) -> Result<SleepStatus, String> {
    let _guard = state.operation_lock.clone().lock_owned().await;
    read_in_blocking_task().await
}

#[tauri::command]
async fn set_sleep_disabled(
    disabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<ChangeOutcome, String> {
    let _guard = state.operation_lock.clone().lock_owned().await;
    set_in_blocking_task(disabled).await
}

async fn refresh_and_emit_if_available(app: AppHandle, operation_lock: Arc<Mutex<()>>) {
    let Ok(_guard) = operation_lock.try_lock_owned() else {
        return;
    };

    match read_in_blocking_task().await {
        Ok(status) => {
            let _ = app.emit("sleep-state-updated", status);
        }
        Err(error) => {
            let _ = app.emit("sleep-state-error", error);
        }
    }
}

fn request_refresh(app: &AppHandle, operation_lock: &Arc<Mutex<()>>) {
    let app = app.clone();
    let operation_lock = operation_lock.clone();
    tauri::async_runtime::spawn(refresh_and_emit_if_available(app, operation_lock));
}

fn show_main_window(app: &AppHandle, operation_lock: &Arc<Mutex<()>>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        request_refresh(app, operation_lock);
    }
}

fn toggle_main_window(app: &AppHandle, operation_lock: &Arc<Mutex<()>>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        show_main_window(app, operation_lock);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_sleep_state,
            set_sleep_disabled
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let operation_lock = Arc::new(Mutex::new(()));
            app.manage(AppState {
                operation_lock: operation_lock.clone(),
            });

            let app_handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.center();

                let focus_app = app_handle.clone();
                let focus_lock = operation_lock.clone();
                window.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Focused(true)) {
                        request_refresh(&focus_app, &focus_lock);
                    }
                });
            }
            show_main_window(&app_handle, &operation_lock);

            let worker_app = app_handle.clone();
            let worker_lock = operation_lock.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(SYNC_INTERVAL);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                loop {
                    interval.tick().await;
                    refresh_and_emit_if_available(worker_app.clone(), worker_lock.clone()).await;
                }
            });

            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let show = MenuItem::with_id(app, "show", "表示", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let menu_lock = operation_lock.clone();
            let tray_lock = operation_lock.clone();
            TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray-icon.png"
                ))?)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    if event.id == "quit" {
                        app.exit(0);
                    } else if event.id == "show" {
                        show_main_window(app, &menu_lock);
                    }
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main_window(tray.app_handle(), &tray_lock);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Lid Awakeの起動に失敗しました");
}
