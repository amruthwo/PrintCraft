//! PrintCraft's Google Images window.
//!
//! Google only answers real web browsers, so picture search runs in the
//! computer's own web engine (WebKitGTK on Linux, WebView2 on Windows,
//! WebKit on macOS) inside a PrintCraft window. A small button appears on
//! any picture the pointer is over; clicking it sends the picture to
//! PrintCraft, which puts it on the page.
//!
//! Started by PrintCraft as `printcraft-web <address>`. Each chosen picture
//! is written to standard output as one line of JSON:
//! `{"src": "<https address or data: address>", "alt": "<description>"}`.
//! The window closes by itself when PrintCraft does (standard input ends).
//!
//! This is a separate program so PrintCraft itself starts even on a
//! computer without the web engine; it then falls back to the browser.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};

#[cfg(not(target_arch = "wasm32"))]
use tao::event::{Event, WindowEvent};
#[cfg(not(target_arch = "wasm32"))]
use tao::event_loop::{ControlFlow, EventLoop};
#[cfg(not(target_arch = "wasm32"))]
use tao::window::{Icon, WindowBuilder};
#[cfg(not(target_arch = "wasm32"))]
use wry::WebViewBuilder;

#[cfg(not(target_arch = "wasm32"))]
/// Runs on every page: the "Add to my project" button, a short tip and a
/// confirmation. Only the main page, not frames inside it.
const PAGE_SCRIPT: &str = r#"
(() => {
  if (window.top !== window || window.__printcraft) return;
  window.__printcraft = true;
  const ready = (f) => document.readyState === 'loading' ? document.addEventListener('DOMContentLoaded', f) : f();
  ready(() => {
    const css = document.createElement('style');
    css.textContent = `
      #pc-add{position:fixed;z-index:2147483647;display:none;padding:8px 14px;border:0;border-radius:18px;background:#2f8fd8;color:#fff;font:600 15px system-ui,sans-serif;box-shadow:0 2px 8px rgba(0,0,0,.35);cursor:pointer}
      #pc-add:hover{background:#1f76bb}
      #pc-tip{position:fixed;z-index:2147483646;left:12px;bottom:12px;max-width:440px;padding:10px 14px 10px 14px;border-radius:12px;background:#fff8e1;color:#333;font:14px/1.35 system-ui,sans-serif;box-shadow:0 2px 10px rgba(0,0,0,.25)}
      #pc-tip b{color:#1f76bb}
      #pc-tip span{float:right;margin-left:10px;cursor:pointer;color:#777}
      #pc-done{position:fixed;z-index:2147483647;left:50%;top:18px;transform:translateX(-50%);padding:10px 18px;border-radius:14px;background:#2e9d5b;color:#fff;font:600 15px system-ui,sans-serif;display:none;box-shadow:0 2px 10px rgba(0,0,0,.3)}`;
    document.head.appendChild(css);
    const btn = document.createElement('button');
    btn.id = 'pc-add';
    btn.textContent = '＋ Add to my project';
    const tip = document.createElement('div');
    tip.id = 'pc-tip';
    tip.innerHTML = '<span title="Hide">✕</span>Point at a picture and click <b>＋ Add to my project</b>. For a sharper copy, click the picture first to open it larger.';
    tip.querySelector('span').onclick = () => tip.remove();
    const done = document.createElement('div');
    done.id = 'pc-done';
    document.body.append(btn, tip, done);
    let target = null;
    const under = (x, y) => {
      for (const el of document.elementsFromPoint(x, y)) {
        if (el === btn) return target;
        if (el.tagName === 'IMG') {
          const r = el.getBoundingClientRect();
          if (r.width >= 60 && r.height >= 60) return el;
        }
      }
      return null;
    };
    document.addEventListener('mousemove', (e) => {
      const img = under(e.clientX, e.clientY);
      if (img && img === target) return;
      target = img;
      if (!img) { btn.style.display = 'none'; return; }
      const r = img.getBoundingClientRect();
      btn.style.left = Math.max(4, r.left + 6) + 'px';
      btn.style.top = Math.max(4, r.top + 6) + 'px';
      btn.style.display = 'block';
    }, true);
    window.addEventListener('scroll', () => { btn.style.display = 'none'; target = null; }, true);
    const send = (img) => {
      const src = img.currentSrc || img.src || '';
      if (!src) return;
      window.ipc.postMessage(JSON.stringify({ src, alt: img.alt || '' }));
      done.textContent = 'Added to your project ✓  Pick another, or close this window.';
      done.style.display = 'block';
      clearTimeout(window.__pcTimer);
      window.__pcTimer = setTimeout(() => { done.style.display = 'none'; }, 3500);
    };
    btn.addEventListener('mousedown', (e) => { e.preventDefault(); e.stopPropagation(); }, true);
    btn.addEventListener('click', (e) => { e.preventDefault(); e.stopPropagation(); if (target) send(target); }, true);
  });
})();
"#;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let url = std::env::args().nth(1).unwrap_or_else(|| "https://images.google.com/".into());
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        eprintln!("usage: printcraft-web <web address>");
        std::process::exit(2);
    }
    // Close along with PrintCraft: it holds our standard input open.
    std::thread::spawn(|| {
        let mut buf = [0u8; 256];
        let mut input = std::io::stdin();
        while matches!(input.read(&mut buf), Ok(n) if n > 0) {}
        std::process::exit(0);
    });

    let event_loop = EventLoop::new();
    let icon = image::load_from_memory(include_bytes!("../../assets/icon_light.png")).ok().and_then(|i| {
        let i = i.thumbnail(64, 64).to_rgba8();
        Icon::from_rgba(i.as_raw().clone(), i.width(), i.height()).ok()
    });
    let window = match WindowBuilder::new().with_title("PrintCraft – Google Images").with_inner_size(tao::dpi::LogicalSize::new(1120.0, 820.0)).with_window_icon(icon).build(&event_loop) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("couldn't open the window: {e}");
            std::process::exit(1);
        }
    };

    // Self-test for development: pick the first picture after the page
    // settles, then quit (PRINTCRAFT_WEB_SELFTEST=1).
    let selftest = std::env::var("PRINTCRAFT_WEB_SELFTEST").is_ok_and(|v| v == "1");
    let mut script = PAGE_SCRIPT.to_string();
    if selftest {
        script.push_str(
            "setTimeout(() => { const i = [...document.images].find(i => i.getBoundingClientRect().width >= 60); \
             window.ipc.postMessage(JSON.stringify(i ? { src: i.currentSrc || i.src, alt: i.alt || '' } : { none: document.title, at: location.href, imgs: document.images.length, text: (document.body ? document.body.innerText : '').slice(0, 300) })); }, 10000);",
        );
    }
    // Keep Google's cookies (its consent choice and "I'm not a robot"
    // check) between uses, in PrintCraft's own data folder.
    let data = directories::ProjectDirs::from("org", "PrintCraft", "PrintCraft").map(|d| d.data_dir().join("web"));
    if let Some(d) = &data {
        let _ = std::fs::create_dir_all(d);
    }
    let mut context = wry::WebContext::new(data);
    let builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(url)
        .with_initialization_script(script)
        .with_ipc_handler(move |req| {
            let body = req.body();
            // Pass on only well-formed picks.
            if selftest {
                println!("selftest: {}", body.chars().take(600).collect::<String>());
                std::process::exit(0);
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(body)
                && v["src"].as_str().is_some_and(|s| s.starts_with("https://") || s.starts_with("http://") || s.starts_with("data:image/"))
            {
                let mut out = std::io::stdout().lock();
                let _ = writeln!(out, "{v}");
                let _ = out.flush();
            }
        })
        // Links that would open another window ("Visit") stay out.
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny);

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(&window);
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        match window.default_vbox() {
            Some(vbox) => builder.build_gtk(vbox),
            None => {
                eprintln!("no window area for the web view");
                std::process::exit(1);
            }
        }
    };
    let _webview = match webview {
        Ok(w) => w,
        Err(e) => {
            eprintln!("the web view couldn't start: {e}");
            std::process::exit(1);
        }
    };

    event_loop.run(move |event, _, flow| {
        // The web context lives as long as the window.
        let _ = &context;
        *flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *flow = ControlFlow::Exit;
        }
    });
}

/// There is no companion window in the browser version.
#[cfg(target_arch = "wasm32")]
fn main() {}
