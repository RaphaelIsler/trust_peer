#[cfg(any(feature = "desktop", feature = "mobile"))]
mod p2p_test;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use p2p_test::P2PTestComponent;

#[cfg(any(feature = "desktop", feature = "mobile"))]
mod barcode_scanner;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use barcode_scanner::BarcodeScanner;

#[cfg(feature = "frontend")]
mod barcode_display;
#[cfg(feature = "frontend")]
pub use barcode_display::BarcodeDisplay;

mod error_overlay;
pub use error_overlay::{send_or_error, ErrorOverlay, Errors};

#[cfg(any(feature = "desktop", feature = "mobile"))]
mod barcode_reader;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use barcode_reader::BarcodeReader;
