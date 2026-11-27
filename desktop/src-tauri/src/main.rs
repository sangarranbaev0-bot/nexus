// Без консольного окна в релизе на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! NexusChat для Windows: окно с веб-клиентом с сервера (адрес в `build.frontendDist`)
//! и проверка обновлений самой оболочки. Интерфейс обновляется вместе с сервером, без переустановки.
//!
//! При запуске показывается мини-окно загрузки (как у Discord): оно проверяет обновление,
//! спрашивает разрешение и показывает прогресс, а потом открывает основное окно.
//!
//! Ни одной странице не выдано разрешений Tauri (нет `capabilities/`). Мини-окно — локальная
//! страница `splash.html`, вшитая в программу; её кнопки — ссылки `/action/...`,
//! переход по которым перехватывает Rust-код ниже.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::mpsc;

/// Приложение часто держат открытым сутками, поэтому проверяем не только при запуске.
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// Если сеть медленная, не держим человека на заставке дольше этого.
const STARTUP_CHECK_TIMEOUT: Duration = Duration::from_secs(8);
/// Чтобы заставка не мигала, показываем её хотя бы столько.
const SPLASH_MIN: Duration = Duration::from_millis(1200);

const SPLASH_HTML: &str = include_str!("../splash.html");

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Update,
    Later,
    Quit,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("splash", |_ctx, _req| {
            tauri::http::Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(SPLASH_HTML.as_bytes().into())
                .expect("splash response")
        })
        .setup(|app| {
            let app = app.handle().clone();
            let (tx, rx) = mpsc::unbounded_channel::<Action>();
            let splash = match open_splash(&app, tx) {
                Ok(w) => Some(w),
                Err(e) => {
                    eprintln!("splash failed: {e}");
                    show_main(&app);
                    None
                }
            };
            tauri::async_runtime::spawn(async move {
                let mut declined: Option<String> = None;
                if let Some(splash) = splash {
                    startup(&app, &splash, rx, &mut declined).await;
                }
                loop {
                    tokio::time::sleep(CHECK_EVERY).await;
                    if let Err(e) = check_for_update(&app, &mut declined).await {
                        // Нет сети или релиза — не беда, попробуем при следующей проверке.
                        eprintln!("update check failed: {e}");
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NexusChat");
}

fn open_splash(app: &AppHandle, tx: mpsc::UnboundedSender<Action>) -> tauri::Result<WebviewWindow> {
    // На Windows своя схема видна странице как http://splash.localhost/.
    let url: Url = "splash://localhost/".parse().expect("splash url");
    WebviewWindowBuilder::new(app, "splash", WebviewUrl::CustomProtocol(url))
        .title("NexusChat")
        .inner_size(300.0, 360.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .center()
        .focused(true)
        .on_navigation(move |url| {
            let action = match url.path() {
                "/action/update" => Some(Action::Update),
                "/action/later" => Some(Action::Later),
                "/action/quit" => Some(Action::Quit),
                _ => None,
            };
            match action {
                Some(a) => {
                    let _ = tx.send(a);
                    false
                }
                // Разрешаем только саму заставку, никуда больше она уйти не может.
                None => url.host_str().is_some_and(|h| h == "localhost" || h == "splash.localhost"),
            }
        })
        .build()
}

fn js(w: &WebviewWindow, code: String) {
    let _ = w.eval(&code);
}

/// Экран загрузки: проверка → (вопрос → загрузка → перезапуск) → основное окно.
async fn startup(
    app: &AppHandle,
    splash: &WebviewWindow,
    mut rx: mpsc::UnboundedReceiver<Action>,
    declined: &mut Option<String>,
) {
    let started = Instant::now();
    // Даём странице отрисоваться.
    tokio::time::sleep(Duration::from_millis(300)).await;
    js(splash, "nx('check')".into());

    let found: Option<Update> = match app.updater() {
        Ok(u) => match tokio::time::timeout(STARTUP_CHECK_TIMEOUT, u.check()).await {
            Ok(Ok(update)) => update,
            Ok(Err(e)) => {
                eprintln!("update check failed: {e}");
                None
            }
            Err(_) => None,
        },
        Err(e) => {
            eprintln!("updater unavailable: {e}");
            None
        }
    };

    if let Some(update) = found {
        js(
            splash,
            format!("nx('found', {:?}, {:?})", update.version, update.current_version),
        );
        match rx.recv().await.unwrap_or(Action::Later) {
            Action::Quit => {
                app.exit(0);
                return;
            }
            Action::Later => *declined = Some(update.version.clone()),
            Action::Update => {
                if install(splash, &update).await {
                    app.restart();
                }
                js(splash, "nx('error')".into());
                tokio::time::sleep(Duration::from_millis(1500)).await;
            }
        }
    }

    js(splash, "nx('ready')".into());
    if let Some(rest) = SPLASH_MIN.checked_sub(started.elapsed()) {
        tokio::time::sleep(rest).await;
    }
    show_main(app);
    let _ = splash.close();
}

/// Скачивает и ставит обновление, показывая проценты на заставке. true — можно перезапускать.
async fn install(splash: &WebviewWindow, update: &Update) -> bool {
    let w = splash.clone();
    let w2 = splash.clone();
    let (mut got, mut shown) = (0u64, u32::MAX);
    js(splash, "nx('progress', 0, '')".into());
    // Подпись проверяется внутри: файл, подписанный не нашим ключом, не установится.
    let res = update
        .download_and_install(
            move |chunk, total| {
                got += chunk as u64;
                let Some(total) = total.filter(|t| *t > 0) else { return };
                let pct = ((got * 100) / total).min(100) as u32;
                if pct != shown {
                    shown = pct;
                    let mb = format!("{:.1} МБ", total as f64 / 1_048_576.0).replace('.', ",");
                    js(&w, format!("nx('progress', {pct}, {mb:?})"));
                }
            },
            move || js(&w2, "nx('install')".into()),
        )
        .await;
    match res {
        Ok(()) => true,
        Err(e) => {
            eprintln!("update install failed: {e}");
            false
        }
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Проверка во время работы: заставки уже нет, поэтому спрашиваем обычным окном.
/// Отказ от версии не повторяем до следующего запуска.
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

    update.download_and_install(|_, _| {}, || {}).await?;
    app.restart();
}
