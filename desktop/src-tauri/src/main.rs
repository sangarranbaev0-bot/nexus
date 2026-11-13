// Без консольного окна в релизе на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! NexusChat для Windows: окно с веб-клиентом с сервера (адрес в `build.frontendDist`)
//! и проверка обновлений самой оболочки. Интерфейс обновляется вместе с сервером, без переустановки.
//!
//! Странице с сервера не выдано ни одного разрешения Tauri (нет `capabilities/`):
//! обновления проверяет только Rust-код ниже.

use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

/// Приложение часто держат открытым сутками, поэтому проверяем не только при запуске.
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut declined: Option<String> = None;
                loop {
                    if let Err(e) = check_for_update(&app, &mut declined).await {
                        // Нет сети или релиза — не беда, попробуем при следующей проверке.
                        eprintln!("update check failed: {e}");
                    }
                    tokio::time::sleep(CHECK_EVERY).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NexusChat");
}

/// Спрашивает разрешение и ставит обновление. Отказ от версии не повторяем до следующего запуска.
async fn check_for_update(
    app: &AppHandle,
    declined: &mut Option<String>,
) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    if declined.as_deref() == Some(update.version.as_str()) {
        return Ok(());
    }

    let notes = update
        .body
        .as_deref()
        .filter(|b| !b.trim().is_empty())
        .map(|b| format!("\n\nЧто нового:\n{b}"))
        .unwrap_or_default();
    // Ответ ждём через канал, не блокируя поток рантайма, пока окно открыто.
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(format!(
            "Доступна новая версия NexusChat {} (у вас {}).{notes}\n\nОбновить сейчас? Приложение перезапустится.",
            update.version, update.current_version
        ))
        .title("Обновление NexusChat")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom("Обновить".into(), "Позже".into()))
        .show(move |ok| {
            let _ = tx.send(ok);
        });
    let accepted = rx.await.unwrap_or(false);
    if !accepted {
        *declined = Some(update.version.clone());
        return Ok(());
    }

    // Подпись проверяется внутри: файл, подписанный не нашим ключом, не установится.
    update.download_and_install(|_, _| {}, || {}).await?;
    app.restart();
}
