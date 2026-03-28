use dioxus::prelude::*;
use qrcode::{render::svg, QrCode};

/// Renders a QR code for the given JSON string.
/// Use `serde_json::to_string(&data).unwrap()` before passing.
#[component]
pub fn BarcodeDisplay(json: String) -> Element {
    let svg_html = use_memo(move || {
        QrCode::new(json.as_bytes())
            .map(|code| {
                code.render::<svg::Color>()
                    .min_dimensions(250, 250)
                    .build()
            })
            .unwrap_or_else(|_| String::from("<p>QR generation failed</p>"))
    });

    rsx! {
        div {
            class: "barcode-display",
            dangerous_inner_html: "{svg_html}",
        }
    }
}
