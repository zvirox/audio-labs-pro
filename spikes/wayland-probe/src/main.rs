//! Phase 0 blur spike: enumerate Wayland globals and report which
//! background-blur protocols the running compositor actually advertises.
//!
//! This is evidence only. Phase 2 ships a runtime detection module that uses
//! the same global list.

use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, QueueHandle};

struct State {
    globals: Vec<(u32, String, u32)>,
}

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.globals.push((name, interface, version));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let display = conn.display();
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = display.get_registry(&qh, ());

    let mut state = State { globals: vec![] };
    queue.roundtrip(&mut state)?;

    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "?".into());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "?".into());
    println!(
        "session={session} desktop={desktop} globals={}",
        state.globals.len()
    );

    println!("--- blur-relevant globals ---");
    let mut found = false;
    for (name, interface, version) in &state.globals {
        if interface.contains("blur") || interface.contains("background_effect") {
            println!("  {interface} v{version} (name {name})");
            found = true;
        }
    }
    if !found {
        println!("  NONE");
    }

    println!("--- related globals present ---");
    for (_, interface, version) in &state.globals {
        if interface.contains("alpha_modifier")
            || interface.contains("viewporter")
            || interface.contains("fractional_scale")
            || interface.contains("xdg_toplevel")
            || interface == "wl_compositor"
            || interface == "wp_presentation"
            || interface.contains("linux_dmabuf")
        {
            println!("  {interface} v{version}");
        }
    }

    println!("--- verdict ---");
    let kde = state
        .globals
        .iter()
        .any(|(_, i, _)| i == "org_kde_kwin_blur_manager");
    let ext = state
        .globals
        .iter()
        .any(|(_, i, _)| i == "ext_background_effect_manager_v1");
    println!(
        "kde_blur={kde} ext_background_effect={ext} fallback={}",
        !kde && !ext
    );

    Ok(())
}
