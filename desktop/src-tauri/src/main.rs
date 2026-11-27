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

#[cfg(target_os = "windows")]
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
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

/// С этим аргументом Windows запускает приложение при входе: окно не показываем, только значок в трее.
const AUTOSTART_ARG: &str = "--autostart";

/// Красные значки с числом непрочитанных для иконки в панели задач (1–9 и «9+»).
const BADGES: [&[u8]; 10] = [
    include_bytes!("../icons/badges/1.png"),
    include_bytes!("../icons/badges/2.png"),
    include_bytes!("../icons/badges/3.png"),
    include_bytes!("../icons/badges/4.png"),
    include_bytes!("../icons/badges/5.png"),
    include_bytes!("../icons/badges/6.png"),
    include_bytes!("../icons/badges/7.png"),
    include_bytes!("../icons/badges/8.png"),
    include_bytes!("../icons/badges/9.png"),
    include_bytes!("../icons/badges/10.png"),
];

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Update,
    Later,
    Quit,
}

fn main() {
    tauri::Builder::default()
        // Второй запуск (ярлык, автозапуск) не открывает вторую копию, а показывает уже открытую.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("splash", |_ctx, _req| {
            tauri::http::Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(SPLASH_HTML.as_bytes())
                .expect("splash response")
        })
        .setup(|app| {
            let app = app.handle().clone();
            create_main(&app)?;
            first_run_autostart(&app);
            setup_tray(&app)?;

            let autostarted = std::env::args().any(|a| a == AUTOSTART_ARG);
            let (tx, rx) = mpsc::unbounded_channel::<Action>();
            // При автозапуске заставку не показываем: приложение тихо ждёт в трее.
            let splash = if autostarted {
                None
            } else {
                match open_splash(&app, tx) {
                    Ok(w) => Some(w),
                    Err(e) => {
                        eprintln!("splash failed: {e}");
                        show_main(&app);
                        None
                    }
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
        // Крестик прячет окно в трей, как в Discord. Выйти — через меню значка в трее.
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
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
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Главное окно создаём сами (в tauri.conf.json у него "create": false),
/// чтобы следить за заголовком страницы: клиент пишет туда «(3) NexusChat».
fn create_main(app: &AppHandle) -> tauri::Result<()> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .cloned()
        .expect("main window in tauri.conf.json");
    WebviewWindowBuilder::from_config(app, &config)?
        .on_document_title_changed(|window, title| on_title_changed(&window, &title))
        .build()?;
    Ok(())
}

/// Число из заголовка вида «(3) NexusChat»; без скобок — 0.
fn unread_from_title(title: &str) -> u32 {
    title
        .strip_prefix('(')
        .and_then(|rest| rest.split_once(')'))
        .and_then(|(n, _)| n.trim().parse().ok())
        .unwrap_or(0)
}

/// Заголовок окна, значок непрочитанного на панели задач и подсказка у значка в трее.
fn on_title_changed(window: &WebviewWindow, title: &str) {
    let _ = window.set_title(title);
    let unread = unread_from_title(title);
    #[cfg(target_os = "windows")]
    {
        let badge = match unread {
            0 => None,
            n => Image::from_bytes(BADGES[(n.min(10) - 1) as usize]).ok(),
        };
        let _ = window.set_overlay_icon(badge);
        if unread > 0 && !window.is_focused().unwrap_or(true) {
            // Мигнуть кнопкой на панели задач, как при новом сообщении в Discord.
            let _ = window.request_user_attention(Some(tauri::UserAttentionType::Informational));
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = &BADGES;
    if let Some(tray) = window.app_handle().tray_by_id("main") {
        let tip = if unread > 0 {
            format!("NexusChat — непрочитанных: {unread}")
        } else {
            "NexusChat".to_string()
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}

/// При первом запуске включаем запуск вместе с Windows, как Discord. Отключается в меню трея.
fn first_run_autostart(app: &AppHandle) {
    let Ok(dir) = app.path().app_data_dir() else { return };
    let marker = dir.join("autostart-initialized");
    if marker.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&dir);
    if app.autolaunch().enable().is_ok() {
        let _ = std::fs::write(&marker, b"1");
    }
}

/// Значок в трее: левый клик открывает окно, правый — меню.
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Открыть NexusChat", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Запускать вместе с Windows",
        true,
        app.autolaunch().is_enabled().unwrap_or(false),
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Выйти", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &autostart, &separator, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("NexusChat")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "autostart" => {
                let launcher = app.autolaunch();
                let was_on = launcher.is_enabled().unwrap_or(false);
                let _ = if was_on { launcher.disable() } else { launcher.enable() };
                let _ = autostart.set_checked(launcher.is_enabled().unwrap_or(!was_on));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
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
