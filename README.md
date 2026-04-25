<!DOCTYPE html>
<html lang="en">
<head>
<!-- Playables SDK -->
<script>// Playables SDK v1.0.0
// Game lifecycle bridge: rAF-based game-ready detection + event communication
(function() {
  'use strict';

  // Idempotency: skip if already initialized (e.g., server-side injection
  // followed by client-side inject-javascript via the Bloks webview component).
  if (window.playablesSDK) return;

  var HANDLER_NAME = 'playablesGameEventHandler';
  var ANDROID_BRIDGE_NAME = '_MetaPlayablesBridge';
  var RAF_FRAME_THRESHOLD = 3;

  var gameReadySent = false;
  var firstInteractionSent = false;
  var errorSent = false;
  var frameCount = 0;
  var originalRAF = window.requestAnimationFrame;

  // --- Transport Layer ---

  function hasIOSBridge() {
    return !!(window.webkit &&
              window.webkit.messageHandlers &&
              window.webkit.messageHandlers[HANDLER_NAME]);
  }

  function hasAndroidBridge() {
    return !!(window[ANDROID_BRIDGE_NAME] &&
              typeof window[ANDROID_BRIDGE_NAME].postEvent === 'function');
  }

  function isInIframe() {
    return !!(window.parent && window.parent !== window);
  }

  function sendEvent(eventName, payload) {
    var message = {
      type: eventName,
      payload: payload || {},
      timestamp: Date.now()
    };

    if (hasIOSBridge()) {
      try {
        window.webkit.messageHandlers[HANDLER_NAME].postMessage(message);
      } catch (e) { /* ignore */ }
      return;
    }

    if (hasAndroidBridge()) {
    try {
      var p = payload || {};
      p.__secureToken = window.__fbAndroidBridgeAuthToken || '';
      window[ANDROID_BRIDGE_NAME].postEvent(
        eventName,
        JSON.stringify(p)
      );
    } catch (e) { /* ignore */ }
    return;
  }

    if (isInIframe()) {
      try {
        window.parent.postMessage(message, '*');
      } catch (e) { /* ignore */ }
      return;
    }
  }

  // --- rAF Game-Ready Detection ---

  function onFrame() {
    if (gameReadySent) return;

    frameCount++;
    if (frameCount >= RAF_FRAME_THRESHOLD) {
      gameReadySent = true;
      sendEvent('game_ready', {
        frame_count: frameCount,
        detected_at: Date.now()
      });
      return;
    }

    originalRAF.call(window, onFrame);
  }

  if (originalRAF) {
    window.requestAnimationFrame = function(callback) {
      if (!gameReadySent) {
        return originalRAF.call(window, function(timestamp) {
          frameCount++;
          if (frameCount >= RAF_FRAME_THRESHOLD && !gameReadySent) {
            gameReadySent = true;
            sendEvent('game_ready', {
              frame_count: frameCount,
              detected_at: Date.now()
            });
          }
          callback(timestamp);
        });
      }
      return originalRAF.call(window, callback);
    };
  }

  // --- First User Interaction Detection ---

  function setupFirstInteractionDetection() {
    var events = ['touchstart', 'mousedown', 'keydown'];

    function onFirstInteraction() {
      if (firstInteractionSent) return;
      firstInteractionSent = true;
      sendEvent('user_interaction_start', null);

      for (var i = 0; i < events.length; i++) {
        document.removeEventListener(events[i], onFirstInteraction, true);
      }
    }

    for (var i = 0; i < events.length; i++) {
      document.addEventListener(events[i], onFirstInteraction, true);
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', setupFirstInteractionDetection);
  } else {
    setupFirstInteractionDetection();
  }

  // --- Auto Error Capture ---

  window.addEventListener('error', function(event) {
    if (errorSent) return;
    errorSent = true;
    sendEvent('error', {
      message: event.message || 'Unknown error',
      source: event.filename || '',
      lineno: event.lineno || 0,
      colno: event.colno || 0,
      auto_captured: true
    });
  });

  window.addEventListener('unhandledrejection', function(event) {
    if (errorSent) return;
    errorSent = true;
    var reason = event.reason;
    sendEvent('error', {
      message: (reason instanceof Error) ? reason.message : String(reason),
      type: 'unhandled_promise_rejection',
      auto_captured: true
    });
  });

  // --- Public API ---

  window.playablesSDK = {
    complete: function(score) {
      sendEvent('game_ended', {
        score: score,
        completed: true
      });
    },

    error: function(message) {
      if (errorSent) return;
      errorSent = true;
      sendEvent('error', {
        message: message || 'Unknown error',
        auto_captured: false
      });
    },

    sendEvent: function(eventName, payload) {
      if (!eventName || typeof eventName !== 'string') return;
      sendEvent(eventName, payload);
    }
  };

  // Kick off rAF detection in case no game code calls rAF immediately
  if (originalRAF) {
    originalRAF.call(window, onFrame);
  }
})();</script>
<script>window.Intl=window.Intl||{};Intl.t=function(s){return(Intl._locale&&Intl._locale[s])||s;};</script>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>fe_ui README — Ferrum</title>
<style>
  :root {
    --bg: #050507;
    --panel: #0d1117;
    --border: #21262d;
    --text: #c9d1d9;
    --muted: #8b949e;
    --accent: #7c3aed;
  }
  * { box-sizing: border-box; }
  html, body { height: 100%; }
  body {
    margin: 0;
    background: radial-gradient(1000px 600px at 85% -10%, #1a1f3a 0%, transparent 60%),
                radial-gradient(800px 500px at -15% 20%, #1a1830 0%, transparent 55%),
                var(--bg);
    color: var(--text);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    -webkit-font-smoothing: antialiased;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 24px;
  }
  .wrap {
    width: 100%;
    max-width: 1024px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    padding: 0 4px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 12px;
    letter-spacing: .08em;
    text-transform: uppercase;
    font-weight: 600;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: linear-gradient(180deg, #a78bfa, #7c3aed);
    box-shadow: 0 0 16px rgba(124,58,237,.6);
  }
  .btn {
    appearance: none;
    border: 1px solid var(--border);
    background: linear-gradient(180deg, #151b23, #0f141b);
    color: var(--text);
    padding: 8px 14px;
    border-radius: 10px;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    transition: transform .08s ease, border-color .2s ease, background .2s ease;
    user-select: none;
  }
  .btn:hover { border-color: #30363d; background: linear-gradient(180deg, #1a212c, #121823); }
  .btn:active { transform: translateY(1px) scale(.99); }
  .btn svg { opacity: .9; }
  .card {
    background: rgba(13,17,23,.85);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 10px 30px rgba(0,0,0,.45), inset 0 1px 0 rgba(255,255,255,.02);
    overflow: hidden;
    backdrop-filter: saturate(120%) blur(8px);
  }
  pre {
    margin: 0;
    padding: 24px;
    overflow: auto;
    white-space: pre;
    font-size: 13px;
    line-height: 1.75;
    color: var(--text);
    tab-size: 2;
  }
  pre::-webkit-scrollbar { height: 10px; width: 10px; }
  pre::-webkit-scrollbar-track { background: transparent; }
  pre::-webkit-scrollbar-thumb { background: #1f2630; border-radius: 8px; border: 2px solid var(--panel); }
  pre::-webkit-scrollbar-thumb:hover { background: #2a3340; }
  .hint {
    color: var(--muted);
    font-size: 11px;
    padding: 10px 24px 18px;
    border-top: 1px dashed #1e242d;
    background: linear-gradient(0deg, rgba(255,255,255,.012), transparent);
  }
  @media (max-width: 640px) {
    body { padding: 12px; }
    pre { padding: 16px; font-size: 12px; }
    .hint { padding: 10px 16px 16px; }
  }
</style>
</head>
<body>
  <div class="wrap">
    <div class="toolbar">
      <div class="title"><span class="dot"></span> README.md — fe_ui</div>
      <button class="btn" id="copyBtn" aria-label="Copy markdown to clipboard">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
        <span>Copy</span>
      </button>
    </div>
    <div class="card">
<pre id="readme"># Ferrum `fe_ui`

**GPU-accelerated Rust UI framework with physics-native layout.**

`fe_ui` treats your interface like a game world. Layout via Taffy, rendering via wgpu, physics via Rapier2D. Every widget has mass, every interaction has momentum. Down to the metal, everywhere.

[![Crates.io](https://img.shields.io/crates/v/fe_ui.svg)](https://crates.io/crates/fe_ui)
[![Docs](https://docs.rs/fe_ui/badge.svg)](https://docs.rs/fe_ui)
[![License](https://img.shields.io/crates/l/fe_ui.svg)](./LICENSE)

&gt; **Status: Experimental** — We’re in early alpha. APIs will break. Physics will be janky. That’s the fun part.

### Why Ferrum?

Modern UI feels dead. Clicks pop. Scrolls slide. Ferrum makes it *alive*.

| | egui | Iced | Dioxus | Flutter | **fe_ui** |
| --- | --- | --- | --- |
| GPU Rendering | ✅ | ✅ | ✅ | ✅ | **✅ wgpu** |
| Layout | Immed | Elm | Taffy | Custom | **Taffy** |
| Physics | ❌ | ❌ | ❌ | ❌ | **✅ Rapier2D** |
| Feel | Tool | App | App | App | **Game** |

**Core Idea**: Taffy decides where things *should* be. Rapier decides where things *are*. A constraint solver makes them agree via springs. Result: UI that bounces, collides, and settles instead of teleporting.

### Features

- **Down to the Metal**: `wgpu` backend → Vulkan, Metal, DX12, WebGPU. One codebase, all platforms.
- **Layout That Doesn’t Suck**: Full Flexbox + Grid + Block via `taffy`. CSS-level power, Rust-level speed.
- **Physics by Default**: `Rapier2D` built-in. Buttons have mass. Lists have friction. Scroll has real inertia.
- **Constraint Solver**: Layout goals become soft spring joints. Resize a window and watch widgets glide to position instead of snapping.
- **Zero-cost Reactivity**: Fine-grained signals. No VDOM. Update 1 text node = 1 draw call, not a tree diff.
- **SDF Rendering**: Text, shapes, shadows = signed distance fields. Crisp at 8K, 60fps on mobile.

### Quick Start

```toml
[dependencies]
fe_ui = "0.1.0-alpha"
```

```rust
use fe_ui::prelude::*;

fn main() {
    App::new()
       .add_window(Window::default())
       .run(ui);
}

#[component]
fn ui(cx: Scope) -&gt; Element {
    let count = use_signal(cx, || 0);

    render! {
        Flex {
            direction: Column,
            gap: 16.0,
            padding: 32.0,
            physics_world: PhysicsWorld::new(Vec2::new(0.0, 900.0)), // gravity

            Text { size: 32.0, "Count: {count}" }

            Button {
                onclick: move |_| count += 1,
                physics: Physics::dynamic()
                   .mass(1.0)
                   .restitution(0.7), // bouncy
                "Click me"
            }
        }
    }
}
```

Click the button. It physically jumps. That's Ferrum.

### How It Works

1. **Layout**: `taffy` computes target positions from Flexbox/Grid.
2. **Constraints**: Targets become spring/motors in `rapier2d`.
3. **Physics**: Step the world at 240Hz. Bodies move toward targets with forces.
4. **Render**: `wgpu` draws SDF quads at interpolated body positions.

No fighting. No snapping. Layout and physics cooperate through the constraint solver.

### Project Structure

```
fe_ui/
├── crates/
│   ├── fe_ui_core/     # Core types, signals, components. No GPU.
│   ├── fe_ui_taffy/    # Taffy integration + layout caching
│   ├── fe_ui_rapier/   # Rapier integration + constraint solver
│   ├── fe_ui_wgpu/     # wgpu renderer, SDF materials, atlas
│   └── fe_ui/          # Main crate, prelude, macros
├── examples/           # `cargo run --example bouncing_button`
└── README.md
```

### Platform Support

| Platform | Status | Backend |
| --- | --- | --- |
| Windows | ✅ | DX12 |
| macOS | ✅ | Metal |
| Linux | ✅ | Vulkan |
| Web | 🚧 | WebGPU/Wasm |
| iOS | 📝 Planned | Metal |
| Android | 📝 Planned | Vulkan |

Windowing via `winit`. Accessibility via `accesskit`.

### Roadmap

**0.1.0 - "Hello Bounce"**
- [x] winit + wgpu window
- [x] Taffy layout → positioned quads
- [ ] Rapier constraint solver MVP
- [ ] SDF rounded rect + text via `cosmic-text`

**0.2.0 - "Actually Usable"**
- [ ] Input + focus system
- [ ] Scroll with physics momentum
- [ ] Hot reload wgsl + view code
- [ ] DevTools: physics debugger

**1.0.0 - "Game-feel UI"**
- [ ] Stable API
- [ ] Mobile targets
- [ ] Editor / visual layout tool

### Contributing

This is early days. The best way to contribute is to break things.

1. Check [Issues](https://github.com/ferrum-ui/fe_ui/issues) for `good-first-physics-bug`
2. Read `ARCHITECTURE.md` to understand the constraint solver
3. Join Discord: [link] — we argue about spring coefficients

### Philosophy

1. **Feel &gt; Features**: 60fps is the minimum. &lt;8ms input latency is the goal.
2. **Physics is UX**: iOS didn’t add springs for fun. Motion communicates.
3. **Native or Nothing**: No webviews. No JS. If it can't run on a Raspberry Pi, it doesn't ship.
4. **Compile Times Matter**: `fe_ui_core` compiles in &lt;3s. Keep the hot path macro-free.

### License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

**Ferrum** = Latin for iron. **fe_ui** = ironclad UI.

Built different.</pre>
      <div class="hint">Plain text • monospace • dark • select all with Ctrl/Cmd+A</div>
    </div>
  </div>

<script>
  (function() {
    const btn = document.getElementById('copyBtn');
    const pre = document.getElementById('readme');
    const original = btn.innerHTML;

    async function copy() {
      const text = pre.textContent;
      try {
        await navigator.clipboard.writeText(text);
        done();
      } catch (e) {
        // Fallback
        const range = document.createRange();
        range.selectNodeContents(pre);
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(range);
        try { document.execCommand('copy'); done(); } catch {}
        sel.removeAllRanges();
      }
    }

    function done() {
      btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg><span>Copied!</span>';
      setTimeout(() => { btn.innerHTML = original; }, 1800);
    }

    btn.addEventListener('click', copy);
  })();
</script>
</body>
</html>
