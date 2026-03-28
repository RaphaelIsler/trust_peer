use super::barcode_scanner::BarcodeScanner;
use dioxus::prelude::*;

/// Wraps `BarcodeScanner` and forwards the raw scanned string.
/// The caller is responsible for parsing the JSON payload.
#[component]
pub fn BarcodeReader(
    on_scan: EventHandler<String>,
    #[props(default)] on_error: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        BarcodeScanner { on_scan, on_error }
    }
}
