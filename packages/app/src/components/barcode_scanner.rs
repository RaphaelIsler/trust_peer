use crate::i18n::{use_i18n, Key};
use dioxus::document::eval;
use dioxus::prelude::*;

/// JS injected into the WebView.
/// Uses @zxing/library (CDN) + getUserMedia to continuously scan for barcodes/QR codes.
/// Results are pushed back to Rust via `dioxus.send(text)`.
const SCANNER_JS: &str = r#"
(async () => {
    const MODULE_URL = 'https://cdn.jsdelivr.net/npm/@zxing/library@0.20.0/umd/index.min.js';

    // Load ZXing UMD bundle if not already present
    if (!window.__zxing_loaded) {
        await new Promise((resolve, reject) => {
            const s = document.createElement('script');
            s.src = MODULE_URL;
            s.onload = resolve;
            s.onerror = () => reject(new Error('Failed to load ZXing from CDN'));
            document.head.appendChild(s);
        });
        window.__zxing_loaded = true;
    }

    const video = document.getElementById('barcode-video');
    if (!video) {
        dioxus.send(JSON.stringify({ error: 'video element not found' }));
        return;
    }

    // Clean up any running scanner
    if (window.__barcode_controls) {
        try { window.__barcode_controls.stop(); } catch(_) {}
        window.__barcode_controls = null;
    }

    const reader = new ZXing.BrowserMultiFormatReader();
    let lastCode = '';
    let lastTime = 0;
    const DEBOUNCE_MS = 1500;

    // decodeFromConstraints: asks for rear camera on mobile, starts internal scan loop
    const constraints = { video: { facingMode: { ideal: 'environment' } } };
    try {
        // The callback fires on every positive decode result
        const controls = await reader.decodeFromConstraints(
            constraints,
            video,
            (result, err) => {
                if (!result) return;
                const code = result.getText();
                const now  = Date.now();
                if (code && (code !== lastCode || now - lastTime > DEBOUNCE_MS)) {
                    lastCode = code;
                    lastTime = now;
                    dioxus.send(JSON.stringify({ code }));
                }
            }
        );
        window.__barcode_controls = controls;
    } catch (e) {
        const msg = e && e.message ? e.message : String(e);
        dioxus.send(JSON.stringify({
            error: msg.includes('Permission') || msg.includes('NotAllowed')
                ? 'camera_denied: ' + msg
                : 'scanner_error: ' + msg
        }));
    }
})();
"#;

const STOP_JS: &str = r#"
    if (window.__barcode_controls) {
        try { window.__barcode_controls.stop(); } catch(_) {}
        window.__barcode_controls = null;
    }
"#;

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ScanEvent {
    Code { code: String },
    Error { error: String },
}

/// Barcode/QR-Code Scanner via WebView + @zxing/library.
///
/// Renders a `<video>` element and starts the camera on button press.
/// Calls `on_scan` with the decoded string whenever a code is detected.
/// Calls `on_error` on camera permission denial or other failures.
///
/// # Example
/// ```rust
/// BarcodeScanner {
///     on_scan:  move |code: String| { log::info!("Scanned: {code}") },
///     on_error: move |err: String|  { log::error!("Scanner error: {err}") },
/// }
/// ```
#[component]
pub fn BarcodeScanner(
    on_scan: EventHandler<String>,
    #[props(default)] on_error: Option<EventHandler<String>>,
) -> Element {
    let i18n = use_i18n();
    let mut active = use_signal(|| false);
    let mut last_code = use_signal(|| String::new());

    let start = move |_| {
        active.set(true);
        spawn(async move {
            // Small delay so the DOM has time to render the <video> element
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            let mut ev = eval(SCANNER_JS);
            loop {
                match ev.recv::<serde_json::Value>().await {
                    Ok(val) => {
                        match serde_json::from_value::<ScanEvent>(val) {
                            Ok(ScanEvent::Code { code }) => {
                                last_code.set(code.clone());
                                on_scan.call(code);
                            }
                            Ok(ScanEvent::Error { error }) => {
                                log::error!("BarcodeScanner: {error}");
                                active.set(false);
                                if let Some(ref handler) = on_error {
                                    handler.call(error);
                                }
                                break;
                            }
                            Err(e) => {
                                log::warn!("BarcodeScanner: unexpected message: {e}");
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    };

    let stop = move |_| {
        active.set(false);
        last_code.set(String::new());
        let _ = eval(STOP_JS);
    };

    rsx! {
        div { class: "barcode-scanner",

            if !active() {
                button { class: "btn btn--primary barcode-scanner__start-btn", onclick: start, "{i18n.t(Key::StartCamera)}" }
            } else {
                div { class: "barcode-scanner__viewport",
                    video {
                        id: "barcode-video",
                        class: "barcode-scanner__video",
                        autoplay: true,
                        muted: true,
                        "playsinline": "true",
                    }
                    button { class: "btn btn--danger btn--sm barcode-scanner__stop-btn", onclick: stop, "{i18n.t(Key::Stop)}" }
                }

                if !last_code().is_empty() {
                    p { class: "barcode-scanner__result", "{i18n.t(Key::Detected)} {last_code}" }
                }
            }
        }
    }
}
