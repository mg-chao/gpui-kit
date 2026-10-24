use gpui::{
    Action, App, ClickEvent, Div, InteractiveElement, InvalidKeystrokeError, KeyBinding, Keystroke,
    Modifiers, OngoingScroll, Pixels, Point, Stateful, StatefulInteractiveElement, TouchPhase,
    Window,
};

#[cfg(not(feature = "gpui-fast"))]
pub use gpui::ScrollEvent;
#[cfg(feature = "gpui-fast")]
pub use gpui::ScrollWheelEvent as ScrollEvent;

/// Legacy name retained for Kit consumers on both GPUI backends.
pub type ScrollWheelEvent = ScrollEvent;

/// Keybinding construction using the selected backend's platform API.
pub trait KeyBindingExt: Sized {
    /// Construct a binding, resolving `secondary-` for the app platform.
    ///
    /// Like `KeyBinding::new`, panics for an invalid shortcut or context predicate.
    fn new_for_app<A: Action>(keystrokes: &str, action: A, context: Option<&str>, cx: &App)
    -> Self;
}

impl KeyBindingExt for KeyBinding {
    fn new_for_app<A: Action>(
        keystrokes: &str,
        action: A,
        context: Option<&str>,
        cx: &App,
    ) -> Self {
        #[cfg(feature = "gpui-fast")]
        {
            let _ = cx;
            Self::new(keystrokes, action, context)
        }
        #[cfg(not(feature = "gpui-fast"))]
        {
            let context = context.map(|context| {
                gpui::KeyBindingContextPredicate::parse(context)
                    .expect("valid keybinding context")
                    .into()
            });
            Self::load(keystrokes, Box::new(action), context, false, None, cx)
                .expect("valid app keybinding")
        }
    }
}

/// Dynamic keystroke conversion using the selected backend's platform API.
pub trait KeystrokeExt: Sized {
    /// Parse a shortcut, resolving the secondary modifier for the app platform.
    fn parse_for_app(source: &str, cx: &App) -> Result<Self, InvalidKeystrokeError>;

    /// Format a shortcut for the app platform.
    fn unparse_for_app(&self, cx: &App) -> String;
}

impl KeystrokeExt for Keystroke {
    fn parse_for_app(source: &str, cx: &App) -> Result<Self, InvalidKeystrokeError> {
        #[cfg(feature = "gpui-fast")]
        {
            let _ = cx;
            Self::parse(source)
        }
        #[cfg(not(feature = "gpui-fast"))]
        Self::parse_with_os(source, cx.platform_info().os)
    }

    fn unparse_for_app(&self, cx: &App) -> String {
        #[cfg(feature = "gpui-fast")]
        {
            let _ = cx;
            self.unparse()
        }
        #[cfg(not(feature = "gpui-fast"))]
        self.unparse(cx.platform_info().os)
    }
}

/// Context-aware modifier queries shared by the fork and GPUI Fast backends.
pub trait ModifiersExt {
    /// Whether the platform's Command (macOS) or Control modifier is pressed.
    fn secondary_pressed(&self, cx: &App) -> bool;

    /// The platform's Command (macOS) or Control modifier, without other keys.
    fn secondary_key_for_app(cx: &App) -> Self;
}

impl ModifiersExt for Modifiers {
    fn secondary_pressed(&self, cx: &App) -> bool {
        #[cfg(feature = "gpui-fast")]
        {
            let _ = cx;
            self.secondary()
        }
        #[cfg(not(feature = "gpui-fast"))]
        self.secondary(cx)
    }

    fn secondary_key_for_app(cx: &App) -> Self {
        #[cfg(feature = "gpui-fast")]
        {
            let _ = cx;
            Self::secondary_key()
        }
        #[cfg(not(feature = "gpui-fast"))]
        Self::secondary_key(cx)
    }
}

/// gpui delimits scroll gestures with `std::time::Instant`, which is
/// unimplemented on wasm32: the first wheel event over an axis-locked scroll
/// area panics with "time not implemented on this platform" and takes the whole
/// application down, leaving the canvas unresponsive. Losing the axis lock in
/// the browser is by far the lesser cost, so the locks below are no-ops there.
pub trait OngoingScrollExt {
    /// Locks a wheel delta to the axis its gesture started on, where the
    /// platform supports it.
    fn lock_axis(&mut self, delta: &mut Point<Pixels>, touch_phase: TouchPhase);
}

impl OngoingScrollExt for OngoingScroll {
    fn lock_axis(&mut self, delta: &mut Point<Pixels>, touch_phase: TouchPhase) {
        #[cfg(target_family = "wasm")]
        let _ = (delta, touch_phase);
        #[cfg(not(target_family = "wasm"))]
        self.filter(delta, touch_phase);
    }
}

pub trait InteractiveElementExt: InteractiveElement {
    /// Set a scroll listener using the selected backend's event API.
    fn on_scroll_event(
        self,
        listener: impl Fn(&ScrollEvent, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        #[cfg(feature = "gpui-fast")]
        {
            self.on_scroll_wheel(listener)
        }
        #[cfg(not(feature = "gpui-fast"))]
        self.on_scroll(listener)
    }

    /// Locks scrolling to the gesture's dominant axis, where the platform
    /// supports it. See [`OngoingScrollExt`] for why this is a no-op on wasm32.
    fn lock_scroll_axis(self) -> Self
    where
        Self: Sized + StatefulInteractiveElement,
    {
        #[cfg(target_family = "wasm")]
        {
            self
        }
        #[cfg(not(target_family = "wasm"))]
        {
            self.restrict_scroll_to_axis()
        }
    }

    /// Set the listener for a double click event.
    fn on_double_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.interactivity().on_click(move |event, window, cx| {
            if event.click_count() == 2 {
                listener(event, window, cx);
            }
        });
        self
    }
}

impl<E: InteractiveElement> InteractiveElementExt for Stateful<E> {}
impl InteractiveElementExt for Div {}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn secondary_bindings_follow_the_app_platform(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            for shortcut in [
                "secondary-enter",
                "secondary-c",
                "secondary-a",
                "ctrl-k secondary-c",
            ] {
                let binding = KeyBinding::new_for_app(
                    shortcut,
                    crate::actions::Confirm { secondary: true },
                    Some("Input && !Disabled"),
                    cx,
                );
                let keystrokes: Vec<_> = shortcut
                    .split_whitespace()
                    .map(|source| Keystroke::parse_for_app(source, cx).unwrap())
                    .collect();
                assert_eq!(binding.match_keystrokes(&keystrokes), Some(false));
                assert!(keystrokes.last().unwrap().modifiers.secondary_pressed(cx));
            }
        });
    }

    #[gpui::test]
    fn secondary_modifiers_follow_the_app_platform(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let secondary = Modifiers::secondary_key_for_app(cx);
            assert_eq!(secondary.number_of_modifiers(), 1);
            assert!(secondary.secondary_pressed(cx));
            assert!(!Modifiers::default().secondary_pressed(cx));
            assert!(!Modifiers::shift().secondary_pressed(cx));
            let other = if secondary.control {
                Modifiers::command()
            } else {
                Modifiers::control()
            };
            assert!(!other.secondary_pressed(cx));
            let shortcut = String::from("secondary-enter");
            let keystroke = Keystroke::parse_for_app(&shortcut, cx).unwrap();
            assert!(keystroke.modifiers.secondary_pressed(cx));
            assert_eq!(keystroke.key, "enter");
            assert_eq!(
                Keystroke::parse_for_app(&keystroke.unparse_for_app(cx), cx).unwrap(),
                keystroke,
            );
        });
    }
}
