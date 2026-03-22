#[cfg(any(feature = "desktop", feature = "mobile"))]
mod p2p_test;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use p2p_test::P2PTestComponent;

#[cfg(feature = "mobile")]
mod barcode_scanner;
#[cfg(feature = "mobile")]
pub use barcode_scanner::BarcodeScanner;
