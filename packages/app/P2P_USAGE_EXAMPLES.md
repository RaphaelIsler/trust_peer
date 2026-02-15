# Using P2PTestComponent in Your App

Quick reference for integrating and using the P2PTestComponent in your Dioxus application.

## Basic Usage

### Import

```rust
use app::components::P2PTestComponent;
```

### Add to Component

```rust
#[component]
fn TestPage() -> Element {
    rsx! {
        div {
            h1 { "WebRTC Testing" }
            P2PTestComponent {}
        }
    }
}
```

### Run

```bash
cargo run -p app --features desktop
```

## Complete Example Page

```rust
use dioxus::prelude::*;
use app::components::P2PTestComponent;

#[component]
pub fn P2pTestingPage() -> Element {
    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh; background: #f5f5f5;",

            // Header
            header {
                style: "background: #2c3e50; color: white; padding: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1);",
                h1 { style: "margin: 0;", "🚀 P2P WebRTC Testing" }
                p { style: "margin: 10px 0 0 0; opacity: 0.8;",
                    "Test peer-to-peer connections in real-time"
                }
            }

            // Main content
            main {
                style: "flex: 1; overflow: auto; padding: 20px;",
                div {
                    style: "max-width: 1200px; margin: 0 auto;",

                    // Info section
                    section {
                        style: "background: white; padding: 20px; border-radius: 8px; margin-bottom: 20px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                        h2 { "ℹ️ About This Tool" }
                        ul {
                            li { "Test WebRTC peer-to-peer connections" }
                            li { "Send and receive messages over DataChannel" }
                            li { "Monitor connection status in real-time" }
                            li { "Requires a running signaling server" }
                        }
                    }

                    // Setup instructions
                    section {
                        style: "background: white; padding: 20px; border-radius: 8px; margin-bottom: 20px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                        h2 { "🔧 Quick Setup" }
                        p {
                            "1. Start the signaling server: "
                            code { "cargo run --example signaling_server -p p2p_webrtc" }
                        }
                        p {
                            "2. Enter a Room ID (e.g., 'test-room')"
                        }
                        p {
                            "3. Click Connect and wait for status to show 'Connected'"
                        }
                        p {
                            "4. Open another instance and connect to the same room"
                        }
                    }

                    // Test component
                    P2PTestComponent {}
                }
            }

            // Footer
            footer {
                style: "background: #f0f0f0; padding: 15px 20px; border-top: 1px solid #ddd; color: #666; font-size: 0.9em;",
                p { style: "margin: 0;",
                    "For more information, see the "
                    a {
                        href: "#",
                        style: "color: #667eea; text-decoration: none;",
                        "documentation"
                    }
                }
            }
        }
    }
}
```

## With Error Boundary

```rust
#[component]
fn P2pTestPageWithError() -> Element {
    rsx! {
        div {
            style: "padding: 20px;",

            match try_render_p2p() {
                Ok(content) => content,
                Err(e) => {
                    rsx! {
                        div {
                            style: "background: #f8d7da; border: 1px solid #f5c6cb; padding: 15px; border-radius: 4px; color: #721c24;",
                            "Error: {e}"
                        }
                    }
                }
            }
        }
    }
}

fn try_render_p2p() -> Result<Element, String> {
    #[cfg(any(feature = "desktop", feature = "mobile"))]
    {
        Ok(rsx! { P2PTestComponent {} })
    }

    #[cfg(not(any(feature = "desktop", feature = "mobile")))]
    {
        Err("P2P testing is only available on Desktop and Mobile".to_string())
    }
}
```

## With Router Integration

```rust
use dioxus::prelude::*;
use app::components::P2PTestComponent;

#[derive(Routable, Clone)]
enum Route {
    #[route("/")]
    Home {},

    #[route("/p2p-test")]
    P2pTest {},
}

#[component]
fn App() -> Element {
    rsx! { Router::<Route> {} }
}

#[component]
fn Home() -> Element {
    rsx! {
        div {
            h1 { "Trust Peer" }
            a { href: "/p2p-test", "P2P Test" }
        }
    }
}

#[component]
fn P2pTest() -> Element {
    rsx! {
        div {
            a { href: "/", "← Back" }
            P2PTestComponent {}
        }
    }
}
```

## With Layout

```rust
#[component]
fn P2pTestWithLayout() -> Element {
    rsx! {
        div {
            style: "display: grid; grid-template-columns: 200px 1fr; min-height: 100vh;",

            // Sidebar
            aside {
                style: "background: #2c3e50; color: white; padding: 20px;",
                nav {
                    a { href: "/", style: "display: block; padding: 10px; color: white; text-decoration: none;",
                        "Home"
                    }
                    a { href: "/p2p-test", style: "display: block; padding: 10px; background: #667eea; text-decoration: none;",
                        "P2P Test"
                    }
                    a { href: "/docs", style: "display: block; padding: 10px; color: white; text-decoration: none;",
                        "Docs"
                    }
                }
            }

            // Main content
            main {
                style: "padding: 20px;",
                P2PTestComponent {}
            }
        }
    }
}
```

## With Conditional Feature Support

```rust
#[component]
fn FeatureAwareP2p() -> Element {
    #[cfg(any(feature = "desktop", feature = "mobile"))]
    {
        rsx! {
            div {
                h2 { "P2P WebRTC Testing (Desktop/Mobile)" }
                P2PTestComponent {}
            }
        }
    }

    #[cfg(not(any(feature = "desktop", feature = "mobile")))]
    {
        rsx! {
            div {
                style: "background: #fff3cd; border: 1px solid #ffc107; padding: 15px; border-radius: 4px; color: #856404;",
                p {
                    "P2P testing is available on Desktop and Mobile platforms. "
                    "This feature is not available on the Web platform."
                }
            }
        }
    }
}
```

## State Management Integration

If you need to integrate with application state:

```rust
use dioxus::prelude::*;
use app::components::P2PTestComponent;

#[component]
pub fn AppWithP2pState() -> Element {
    // Application state
    let mut app_state = use_signal(|| AppState {
        user_id: "user123".to_string(),
        room: None,
    });

    rsx! {
        div {
            style: "display: flex; gap: 20px; padding: 20px;",

            // User info
            aside {
                style: "width: 200px;",
                div { "User: {app_state.read().user_id}" }
                if let Some(room) = app_state.read().room {
                    div { "Current Room: {room}" }
                }
            }

            // P2P component
            section {
                style: "flex: 1;",
                P2PTestComponent {}
            }
        }
    }
}

#[derive(Clone)]
struct AppState {
    user_id: String,
    room: Option<String>,
}
```

## Styling Customization

```rust
#[component]
fn StyledP2pTest() -> Element {
    rsx! {
        div {
            style: "
                --primary-color: #667eea;
                --success-color: #28a745;
                --danger-color: #dc3545;
                --warning-color: #ffc107;
                --light-bg: #f8f9fa;
                --border-color: #ddd;
            ",

            // Custom wrapper
            div {
                style: "
                    max-width: 1000px;
                    margin: 0 auto;
                    border-radius: 8px;
                    overflow: hidden;
                    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                    background: white;
                ",

                P2PTestComponent {}
            }
        }
    }
}
```

## Testing Configuration

For testing with different servers:

```rust
#[component]
fn P2pTestConfig() -> Element {
    let mut server_url = use_signal(|| "ws://127.0.0.1:9001/ws".to_string());
    let mut show_test = use_signal(|| false);

    rsx! {
        div {
            style: "padding: 20px;",

            // Server config
            div {
                style: "margin-bottom: 20px;",
                label {
                    "Signaling Server: "
                    input {
                        r#type: "text",
                        value: "{server_url}",
                        oninput: move |e| server_url.set(e.value()),
                        style: "width: 300px; padding: 8px; border: 1px solid #ddd;",
                    }
                }
            }

            // Toggle for showing test component
            button {
                onclick: move |_| show_test.toggle(),
                "Toggle Test Component"
            }

            if show_test() {
                div {
                    style: "margin-top: 20px;",
                    P2PTestComponent {}
                }
            }
        }
    }
}
```

## TypeScript/React Developer Migration

If you're coming from React/TypeScript:

```rust
// React-like pattern in Rust
use dioxus::prelude::*;
use app::components::P2PTestComponent;

// Component function (like React component)
#[component]
fn MyComponent(name: String) -> Element {
    // State hook (like useState)
    let mut count = use_signal(|| 0);

    // Effect hook (like useEffect)
    use_effect(move || {
        println!("Component mounted: {}", name);
    });

    // JSX-like RSX
    rsx! {
        div {
            h1 { "Hello, {name}!" }
            p { "Count: {count}" }
            button {
                onclick: move |_| count += 1,
                "Increment"
            }

            // Use component
            P2PTestComponent {}
        }
    }
}
```

## Performance Tips

1. **Memoization** - Use `use_memo` for expensive calculations
2. **Lazy Loading** - Load component only when needed
3. **Message History** - Component handles large message lists efficiently

```rust
// Example: Lazy load component
#[component]
fn LazyP2pTest() -> Element {
    let mut loaded = use_signal(|| false);

    rsx! {
        button {
            onclick: move |_| loaded.set(true),
            "Load P2P Test"
        }

        if loaded() {
            P2PTestComponent {}
        }
    }
}
```

## Accessibility

```rust
#[component]
fn AccessibleP2pTest() -> Element {
    rsx! {
        div {
            role: "region",
            "aria-label": "P2P WebRTC Testing Component",

            h1 { "P2P WebRTC Testing" }
            p { "Test peer-to-peer connections in real-time" }

            P2PTestComponent {}
        }
    }
}
```

---

For more examples and detailed documentation, see:
- [P2P_TEST_COMPONENT.md](P2P_TEST_COMPONENT.md) - Complete component documentation
- [P2P_QUICKSTART.md](P2P_QUICKSTART.md) - Quick start guide
- [p2p_webrtc README](../p2p_webrtc/README.md) - Library documentation
