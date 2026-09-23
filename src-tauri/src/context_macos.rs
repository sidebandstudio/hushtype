//! Native context capture. No Apple events, subprocesses, or clipboard-copy
//! keystrokes. Attribute reads have a deadline and per-message timeout.
use crate::modes::{capture_optional, match_mode, CapturedContext, Mode, ResolvedContext};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSString;
use std::ffi::{c_char, c_void};
use std::time::{Duration, Instant};

type CFRef = *const c_void;
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> CFRef;
    fn AXUIElementCopyAttributeValue(element: CFRef, attribute: CFRef, value: *mut CFRef) -> i32;
    fn AXUIElementSetMessagingTimeout(element: CFRef, seconds: f32) -> i32;
    fn AXUIElementGetTypeID() -> usize;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: CFRef);
    fn CFRetain(value: CFRef) -> CFRef;
    fn CFGetTypeID(value: CFRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFStringGetLength(value: CFRef) -> isize;
    fn CFStringGetCString(value: CFRef, buffer: *mut c_char, size: isize, encoding: u32) -> u8;
    fn CFArrayGetTypeID() -> usize;
    fn CFArrayGetCount(value: CFRef) -> isize;
    fn CFArrayGetValueAtIndex(value: CFRef, index: isize) -> CFRef;
    fn CFURLGetTypeID() -> usize;
    fn CFURLGetString(value: CFRef) -> CFRef;
}

struct Owned(CFRef);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}
impl Owned {
    fn attr(&self, key: &str, deadline: Instant) -> Option<Owned> {
        self.attr_with_timeout(key, deadline, 0.02)
    }
    fn attr_with_timeout(&self, key: &str, deadline: Instant, timeout: f32) -> Option<Owned> {
        if Instant::now() >= deadline {
            return None;
        }
        let key = NSString::from_str(key);
        let mut value = std::ptr::null();
        // SAFETY: each Owned is a retained CF object. AX calls only target
        // application/window/element objects with their type checked below.
        let status = unsafe {
            if CFGetTypeID(self.0) != AXUIElementGetTypeID() {
                return None;
            }
            AXUIElementSetMessagingTimeout(self.0, timeout);
            AXUIElementCopyAttributeValue(self.0, &*key as *const NSString as CFRef, &mut value)
        };
        if status == 0 && !value.is_null() {
            Some(Owned(value))
        } else {
            log::debug!("Accessibility attribute {} unavailable ({status})", key);
            None
        }
    }
    fn text(&self) -> Option<String> {
        unsafe { cf_text(self.0) }
    }
    fn string(&self, key: &str, deadline: Instant) -> Option<String> {
        self.attr(key, deadline)?.text()
    }
    fn children(&self, deadline: Instant) -> Vec<Owned> {
        let Some(array) = self.attr("AXChildren", deadline) else {
            return vec![];
        };
        // SAFETY: check the array type and each borrowed element before
        // retaining it beyond the array's lifetime. Bound traversal work.
        unsafe {
            if CFGetTypeID(array.0) != CFArrayGetTypeID() {
                return vec![];
            }
            (0..CFArrayGetCount(array.0).min(32))
                .filter_map(|i| {
                    let value = CFArrayGetValueAtIndex(array.0, i);
                    if !value.is_null() && CFGetTypeID(value) == AXUIElementGetTypeID() {
                        Some(Owned(CFRetain(value)))
                    } else {
                        None
                    }
                })
                .collect()
        }
    }
}

unsafe fn cf_text(mut value: CFRef) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if CFGetTypeID(value) == CFURLGetTypeID() {
        value = CFURLGetString(value);
    }
    if value.is_null() || CFGetTypeID(value) != CFStringGetTypeID() {
        return None;
    }
    let length = CFStringGetLength(value);
    if !(1..=8192).contains(&length) {
        return None;
    }
    let mut bytes = vec![0_u8; length as usize * 4 + 1];
    if CFStringGetCString(
        value,
        bytes.as_mut_ptr().cast(),
        bytes.len() as isize,
        0x08000100,
    ) == 0
    {
        return None;
    }
    let end = bytes.iter().position(|b| *b == 0)?;
    String::from_utf8(bytes[..end].to_vec()).ok()
}

pub fn frontmost() -> (CapturedContext, i32) {
    objc2::rc::autoreleasepool(|_| {
        let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
            return (CapturedContext::default(), 0);
        };
        (
            CapturedContext {
                app_name: app.localizedName().map(|s| s.to_string()),
                bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
                ..Default::default()
            },
            app.processIdentifier(),
        )
    })
}

/// A clipboard reader elsewhere must not turn an obvious non-text target into
/// a successful paste. Read only the focused element's role, never its text,
/// selection, or value. Unknown/custom controls keep the upstream paste path.
pub fn focused_control_is_nontext() -> bool {
    let (_, pid) = frontmost();
    if pid <= 0 {
        return false;
    }
    // SAFETY: AX returns a retained application element, released by Owned.
    let root = unsafe { AXUIElementCreateApplication(pid) };
    if root.is_null() {
        return false;
    }
    let root = Owned(root);
    let started = Instant::now();
    let deadline = started + Duration::from_millis(200);
    let role = root
        .attr_with_timeout("AXFocusedUIElement", deadline, 0.15)
        .and_then(|focused| focused.string("AXRole", deadline));
    log::debug!(
        "Paste target role: {} ({}ms)",
        role.as_deref().unwrap_or("unavailable"),
        started.elapsed().as_millis()
    );
    matches!(
        role.as_deref(),
        Some(
            "AXWindow"
                | "AXButton"
                | "AXCheckBox"
                | "AXRadioButton"
                | "AXMenuItem"
                | "AXMenuBarItem"
                | "AXStaticText"
                | "AXImage"
                | "AXToolbar"
                | "AXScrollBar"
                | "AXSlider"
        )
    )
}

fn valid_url(value: String) -> Option<String> {
    let url = reqwest::Url::parse(&value).ok()?;
    matches!(url.scheme(), "http" | "https").then_some(value)
}

fn browser_url(window: &Owned, deadline: Instant) -> Option<String> {
    if let Some(url) = window.string("AXDocument", deadline).and_then(valid_url) {
        return Some(url);
    }
    let mut queue = std::collections::VecDeque::from(window.children(deadline));
    let mut visited = 0;
    while let Some(element) = queue.pop_front() {
        if Instant::now() >= deadline || visited >= 96 {
            break;
        }
        visited += 1;
        let role = element.string("AXRole", deadline).unwrap_or_default();
        if role == "AXWebArea" {
            if let Some(url) = element
                .string("AXURL", deadline)
                .and_then(valid_url)
                .or_else(|| element.string("AXDocument", deadline).and_then(valid_url))
            {
                return Some(url);
            }
            // Never walk page content or inspect link targets.
            continue;
        }
        if ["AXTextField", "AXComboBox"].contains(&role.as_str()) {
            let label = element
                .string("AXIdentifier", deadline)
                .unwrap_or_default()
                .to_lowercase();
            let description = element
                .string("AXDescription", deadline)
                .unwrap_or_default()
                .to_lowercase();
            if ["address", "url", "omnibox"]
                .iter()
                .any(|word| label.contains(word) || description.contains(word))
            {
                if let Some(url) = element.string("AXValue", deadline).and_then(valid_url) {
                    return Some(url);
                }
            }
        }
        queue.extend(element.children(deadline));
    }
    None
}

pub fn capture(
    mut context: CapturedContext,
    pid: i32,
    modes: &[Mode],
    clipboard: impl FnOnce() -> Option<String>,
) -> ResolvedContext {
    objc2::rc::autoreleasepool(|_| {
        let deadline = Instant::now() + Duration::from_millis(250);
        let element = if pid > 0 {
            // SAFETY: returns a +1 owned reference for the captured app PID.
            let ptr = unsafe { AXUIElementCreateApplication(pid) };
            (!ptr.is_null()).then(|| Owned(ptr))
        } else {
            None
        };
        if let Some(element) = &element {
            if let Some(window) = element.attr("AXFocusedWindow", deadline) {
                context.window_title = window
                    .string("AXTitle", deadline)
                    .map(|s| s.chars().take(1024).collect());
                if [
                    "com.apple.Safari",
                    "com.google.Chrome",
                    "company.thebrowser.Browser",
                    "com.brave.Browser",
                ]
                .contains(&context.bundle_id.as_deref().unwrap_or_default())
                {
                    context.url = browser_url(&window, deadline);
                }
            }
        }
        let mode = match_mode(modes, &context);
        capture_optional(
            &mut context,
            &mode.context,
            || {
                element
                    .as_ref()?
                    .attr("AXFocusedUIElement", deadline)?
                    .string("AXSelectedText", deadline)
            },
            clipboard,
        );
        ResolvedContext { mode, context }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_capture_reads_the_current_session_without_optional_context() {
        let (identity, pid) = frontmost();
        let expected_bundle = identity.bundle_id.clone();
        let result = capture(identity, pid, &crate::modes::default_modes(), || {
            panic!("clipboard read without opt-in")
        });
        assert_eq!(result.context.bundle_id, expected_bundle);
        assert!(result.context.selected_text.is_none());
        assert!(result.context.clipboard.is_none());
        // Optional expectations turn this into a repeatable native desktop
        // check without capturing or printing unrelated user window content.
        if let Ok(bundle) = std::env::var("HUSHTYPE_CONTEXT_BUNDLE") {
            assert_eq!(result.context.bundle_id.as_deref(), Some(bundle.as_str()));
        }
        if let Ok(url) = std::env::var("HUSHTYPE_CONTEXT_URL") {
            assert_eq!(result.context.url.as_deref(), Some(url.as_str()));
        }
        if let Ok(mode) = std::env::var("HUSHTYPE_CONTEXT_MODE") {
            assert_eq!(result.mode.name, mode);
        }
        println!(
            "Native frontmost probe: bundle={:?}, mode={}, window_available={}, url_available={}",
            result.context.bundle_id,
            result.mode.name,
            result.context.window_title.is_some(),
            result.context.url.is_some()
        );
    }
}
