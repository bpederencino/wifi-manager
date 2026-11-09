//! Shortcuts — keyboard and D-Bus triggered actions (Escape, focus loss, reload).

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::glib;

use crate::ui::window::PanelWidgets;

use super::{AppState, refresh_list};

/// Set up Escape key handler to hide panel (with proper state tracking).
pub(super) fn setup_escape_key(widgets: &PanelWidgets, panel_state: crate::daemon::PanelState) {
    use gtk4::{gdk, glib, prelude::*, EventControllerKey};
    
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            panel_state.hide();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    widgets.window.add_controller(key_controller);
}

/// Hide the panel when it loses keyboard focus, so clicking outside closes it.
///
/// The compositor gives the panel the keyboard when it appears; clicking
/// another window or the desktop takes it away. A row's context menu also
/// takes the keyboard while open, so the panel stays while one is shown.
pub(super) fn setup_hide_on_focus_loss(
    widgets: &PanelWidgets,
    panel_state: crate::daemon::PanelState,
) {
    use gtk4::prelude::*;

    widgets.window.connect_is_active_notify(move |window| {
        if window.is_active() || !window.is_visible() {
            return;
        }
        let window = window.clone();
        let panel_state = panel_state.clone();
        // Give a context menu that just opened the time to show up.
        glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
            if window.is_visible() && !window.is_active() && !has_open_popover(window.upcast_ref())
            {
                panel_state.hide();
            }
        });
    });
}

/// Whether a popover (context menu) is shown anywhere under `widget`.
fn has_open_popover(widget: &gtk4::Widget) -> bool {
    use gtk4::prelude::*;

    let mut child = widget.first_child();
    while let Some(current) = child {
        if (current.is::<gtk4::Popover>() && current.is_visible()) || has_open_popover(&current) {
            return true;
        }
        child = current.next_sibling();
    }
    false
}

/// Poll the reload_requested flag and reload config/CSS when set.
pub(super) fn setup_reload_on_request(
    widgets: &PanelWidgets,
    state: Rc<RefCell<AppState>>,
    reload_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let list_box = widgets.network_list_box.clone();
    let status = widgets.status_label.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        if reload_requested.swap(false, std::sync::atomic::Ordering::Relaxed) {
            log::info!("Reload requested - refreshing network list with new config");
            let state = Rc::clone(&state);
            let list_box = list_box.clone();
            let status = status.clone();

            glib::spawn_future_local(async move {
                // Reload CSS
                crate::ui::window::reload_css();
                // Refresh network list (which will reload config for icons)
                refresh_list(&state, &list_box, &status).await;
            });
        }
        glib::ControlFlow::Continue
    });
}
