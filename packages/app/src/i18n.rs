/// Internationalization support.
///
/// Provides a type-safe translation key system with Dioxus context integration.
/// Add new languages by extending `Lang` and adding a match arm in `translate()`.

// ── Data layer (always compiled) ─────────────────────────────────────────────

/// Supported UI languages.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    En,
    De,
}

/// All translatable UI strings.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // App shell
    AppTitle,
    AddIdentity,

    // Ledger node overview
    Details,
    Less,
    PrivateChainLabel,
    PublicChainLabel,
    Identifications,
    PrivateChainBtn,
    PublicChainBtn,
    LoadingBlockchain,

    // Identification form
    CreateNewIdentity,
    FirstName,
    LastName,
    MiddleName,
    DateOfBirth,
    CreateBtn,

    // Validation messages
    FillRequiredFields,
    InvalidDateFormat,

    // Block entry types and field labels
    EntryVerification,
    EntryIdentification,
    EntryLink,
    EntryCurrentAmount,
    NoEntries,
    NoBlocks,
    Block,
    Entries,
    Salt,
    PublicKey,
    SignatureBytes,
    Bytes,
    Preview,
    Timestamp,
    Amount,

    // Money view mode selector
    MoneyCurrent,
    MoneyStored,

    // Barcode scanner
    StartCamera,
    Stop,
    Detected,
}

/// Returns the translated string for `key` in the given `lang`.
pub fn translate(lang: Lang, key: Key) -> &'static str {
    match lang {
        Lang::En => en(key),
        Lang::De => de(key),
    }
}

fn en(key: Key) -> &'static str {
    match key {
        Key::AppTitle => "TrustPeer",
        Key::AddIdentity => "+ Add Identity",
        Key::Details => "Details",
        Key::Less => "Less",
        Key::PrivateChainLabel => "Private chain:",
        Key::PublicChainLabel => "Public chain:",
        Key::Identifications => "Identifications:",
        Key::PrivateChainBtn => "Private Chain",
        Key::PublicChainBtn => "Public Chain",
        Key::LoadingBlockchain => "Loading blockchain...",
        Key::CreateNewIdentity => "Create New Identity",
        Key::FirstName => "First Name *",
        Key::LastName => "Last Name *",
        Key::MiddleName => "Middle Name (optional)",
        Key::DateOfBirth => "Date of Birth *",
        Key::CreateBtn => "Create",
        Key::FillRequiredFields => "Please fill in all required fields",
        Key::InvalidDateFormat => "Invalid date format. Use YYYY-MM-DD",
        Key::EntryVerification => "Verification",
        Key::EntryIdentification => "Identification",
        Key::EntryLink => "Link",
        Key::EntryCurrentAmount => "Current Amount",
        Key::NoEntries => "No entries",
        Key::NoBlocks => "No blocks",
        Key::Block => "Block",
        Key::Entries => "Entries:",
        Key::Salt => "Salt:",
        Key::PublicKey => "Public:",
        Key::SignatureBytes => "Signature bytes:",
        Key::Bytes => "Bytes:",
        Key::Preview => "Preview:",
        Key::Timestamp => "Timestamp:",
        Key::Amount => "Amount:",
        Key::MoneyCurrent => "current",
        Key::MoneyStored => "stored",
        Key::StartCamera => "Start Camera",
        Key::Stop => "Stop",
        Key::Detected => "Detected:",
    }
}

fn de(key: Key) -> &'static str {
    match key {
        Key::AppTitle => "TrustPeer",
        Key::AddIdentity => "+ Identität hinzufügen",
        Key::Details => "Details",
        Key::Less => "Weniger",
        Key::PrivateChainLabel => "Private Kette:",
        Key::PublicChainLabel => "Öffentliche Kette:",
        Key::Identifications => "Identifikationen:",
        Key::PrivateChainBtn => "Private Kette",
        Key::PublicChainBtn => "Öffentliche Kette",
        Key::LoadingBlockchain => "Blockchain wird geladen...",
        Key::CreateNewIdentity => "Neue Identität erstellen",
        Key::FirstName => "Vorname *",
        Key::LastName => "Nachname *",
        Key::MiddleName => "Zweiter Vorname (optional)",
        Key::DateOfBirth => "Geburtsdatum *",
        Key::CreateBtn => "Erstellen",
        Key::FillRequiredFields => "Bitte alle Pflichtfelder ausfüllen",
        Key::InvalidDateFormat => "Ungültiges Datumsformat. Bitte JJJJ-MM-TT verwenden",
        Key::EntryVerification => "Verifikation",
        Key::EntryIdentification => "Identifikation",
        Key::EntryLink => "Verknüpfung",
        Key::EntryCurrentAmount => "Aktueller Betrag",
        Key::NoEntries => "Keine Einträge",
        Key::NoBlocks => "Keine Blöcke",
        Key::Block => "Block",
        Key::Entries => "Einträge:",
        Key::Salt => "Salt:",
        Key::PublicKey => "Öffentlich:",
        Key::SignatureBytes => "Signatur-Bytes:",
        Key::Bytes => "Bytes:",
        Key::Preview => "Vorschau:",
        Key::Timestamp => "Zeitstempel:",
        Key::Amount => "Betrag:",
        Key::MoneyCurrent => "aktuell",
        Key::MoneyStored => "gespeichert",
        Key::StartCamera => "Kamera starten",
        Key::Stop => "Stoppen",
        Key::Detected => "Erkannt:",
    }
}

// ── Dioxus integration ────────────────────────────────────────────────────────

use dioxus::prelude::*;

/// Handle to the current language, retrieved from Dioxus context.
/// `Copy` because `Signal<T>` is `Copy`.
#[derive(Clone, Copy)]
pub struct I18n {
    pub(crate) lang: Signal<Lang>,
}

impl I18n {
    /// Translate a key using the current language.
    pub fn t(self, key: Key) -> &'static str {
        translate(*self.lang.read(), key)
    }

    /// Returns the active language.
    pub fn lang(self) -> Lang {
        *self.lang.read()
    }

    /// Switch to a different language. Triggers a re-render for all subscribers.
    pub fn set_lang(mut self, lang: Lang) {
        self.lang.set(lang);
    }
}

/// Retrieves the `I18n` handle from Dioxus context.
/// Must be called inside a `#[component]` function.
/// Panics if no language context has been provided (see `provide_i18n_context`).
pub fn use_i18n() -> I18n {
    I18n {
        lang: use_context::<Signal<Lang>>(),
    }
}

/// Registers the language signal in the Dioxus context tree.
/// Detects the OS locale automatically; falls back to English.
/// Call this once in the root `App` component before any child renders.
/// Returns the signal so the root component can use it directly.
pub fn provide_i18n_context() -> Signal<Lang> {
    use_context_provider(|| Signal::new(detect_lang()))
}

/// Detects the preferred language from OS locale environment variables.
/// Checks `LANG`, `LANGUAGE`, and `LC_ALL` in order.
/// Returns the first supported language found, or `Lang::En` as fallback.
fn detect_lang() -> Lang {
    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LANGUAGE"))
        .or_else(|_| std::env::var("LC_ALL"))
        .unwrap_or_default()
        .to_lowercase();

    if locale.starts_with("de") {
        Lang::De
    } else {
        Lang::En
    }
}

/// Language selector dropdown.
/// Reads and writes the language from the Dioxus i18n context.
#[component]
pub fn LanguageSelector() -> Element {
    let i18n = use_i18n();

    rsx! {
        select {
            class: "lang-selector",
            value: match i18n.lang() {
                Lang::En => "en",
                Lang::De => "de",
            },
            onchange: move |e| match e.value().as_str() {
                "de" => i18n.set_lang(Lang::De),
                _ => i18n.set_lang(Lang::En),
            },
            option { value: "en", "English" }
            option { value: "de", "Deutsch" }
        }
    }
}
