//! Native macOS view of [`Controls`]: a floating AppKit HUD panel of platform sliders,
//! checkboxes, text fields and readouts, built from the same model the browser view uses.
//!
//! The panel lives on AppKit's main thread. Widget callbacks only queue the edit; the
//! owner applies queued edits to the model with [`MacPanel::apply_edits`] and refreshes
//! widgets from the model with [`MacPanel::sync`], both from its event loop.

use std::cell::RefCell;

use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained, runtime::AnyObject, sel,
};
use objc2_app_kit::{
    NSBackingStoreType, NSBitmapImageFileType, NSButton, NSColor, NSControl, NSControlStateValueOn,
    NSFont, NSLayoutAttribute, NSPanel, NSScreen, NSSlider, NSStackView, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSDictionary, NSEdgeInsets, NSObject, NSObjectProtocol, NSPoint,
    NSRect, NSSize, NSString,
};

use crate::{ControlId, Controls, Kind, Value};

const LABEL_WIDTH: f64 = 116.0;
const SLIDER_WIDTH: f64 = 190.0;
const VALUE_WIDTH: f64 = 64.0;

enum Edit {
    Number(ControlId, f64),
    Value(ControlId, Value),
}

struct TargetIvars {
    edits: RefCell<Vec<Edit>>,
    kinds: Vec<Kind>,
    wake: Option<Box<dyn Fn()>>,
}

define_class!(
    // SAFETY: NSObject has no subclass invariants; the target has no Drop implementation and
    // AppKit calls it only on the main run loop.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TargetIvars]
    struct PanelTarget;

    // SAFETY: NSObjectProtocol adds no callback signature requirements.
    unsafe impl NSObjectProtocol for PanelTarget {}

    impl PanelTarget {
        // SAFETY: AppKit sends a control action with the sending NSControl.
        #[unsafe(method(controlChanged:))]
        fn control_changed(&self, sender: &NSControl) {
            let ivars = self.ivars();
            let Ok(index) = u16::try_from(sender.tag()) else {
                return;
            };
            let id = ControlId(index);
            let edit = match ivars.kinds.get(id.index()) {
                Some(Kind::Slider { .. }) => Edit::Number(id, sender.doubleValue()),
                Some(Kind::Toggle) => Edit::Value(id, Value::Flag(sender.integerValue() != 0)),
                Some(Kind::Text) => Edit::Value(id, Value::Text(sender.stringValue().to_string())),
                Some(Kind::Readout { .. }) | None => return,
            };
            ivars.edits.borrow_mut().push(edit);
            if let Some(wake) = &ivars.wake {
                wake();
            }
        }
    }
);

impl PanelTarget {
    #[expect(
        unsafe_code,
        reason = "the NSObject initializer is the narrow AppKit action-target bridge"
    )]
    fn new(ivars: TargetIvars, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ivars);
        // SAFETY: NSObject's initializer accepts this allocated subclass.
        unsafe { msg_send![super(this), init] }
    }
}

enum Widget {
    Slider {
        slider: Retained<NSSlider>,
        value: Retained<NSTextField>,
        scale: f64,
    },
    Toggle(Retained<NSButton>),
    Text(Retained<NSTextField>),
    Readout(Retained<NSTextField>),
}

/// A floating AppKit panel presenting one [`Controls`] model.
pub struct MacPanel {
    panel: Retained<NSPanel>,
    target: Retained<PanelTarget>,
    widgets: Vec<Widget>,
    seen: u64,
}

impl MacPanel {
    /// Build a panel titled `title` for every declared control, grouped by key prefix.
    /// `wake` runs after each widget edit so an idle event loop applies it promptly.
    ///
    /// # Errors
    /// Refuses to run off AppKit's main thread.
    pub fn new(
        controls: &Controls,
        title: &str,
        wake: Option<Box<dyn Fn()>>,
    ) -> Result<Self, String> {
        let mtm = MainThreadMarker::new()
            .ok_or("the control panel must be built on AppKit's main thread")?;
        let target = PanelTarget::new(
            TargetIvars {
                edits: RefCell::new(Vec::new()),
                kinds: controls
                    .specs()
                    .iter()
                    .map(|spec| spec.kind().clone())
                    .collect(),
                wake,
            },
            mtm,
        );
        let column = NSStackView::stackViewWithViews(&NSArray::new(), mtm);
        column.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        column.setAlignment(NSLayoutAttribute::Leading);
        column.setSpacing(6.0);
        column.setEdgeInsets(NSEdgeInsets {
            top: 12.0,
            left: 14.0,
            bottom: 14.0,
            right: 14.0,
        });
        let mut widgets: Vec<Option<Widget>> = std::iter::repeat_with(|| None)
            .take(controls.specs().len())
            .collect();
        for (group, ids) in controls.groups() {
            if !group.is_empty() {
                let header =
                    NSTextField::labelWithString(&NSString::from_str(&heading(group)), mtm);
                header.setFont(Some(&NSFont::boldSystemFontOfSize(12.0)));
                header.setTextColor(Some(&NSColor::secondaryLabelColor()));
                column.addArrangedSubview(&header);
            }
            for id in ids {
                let spec = &controls.specs()[id.index()];
                let text = controls.formatted(id);
                let (row, widget) = build_row(
                    spec.label(),
                    spec.kind(),
                    controls.value(id),
                    &text,
                    id,
                    &target,
                    mtm,
                );
                column.addArrangedSubview(&row);
                widgets[id.index()] = Some(widget);
            }
        }
        let widgets: Vec<Widget> = widgets.into_iter().flatten().collect();
        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::UtilityWindow
            | NSWindowStyleMask::HUDWindow;
        let size = column.fittingSize();
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), size),
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        panel.setTitle(&NSString::from_str(title));
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setHidesOnDeactivate(false);
        panel.setContentView(Some(&column));
        panel.setContentSize(NSSize::new(
            size.width.max(LABEL_WIDTH + SLIDER_WIDTH + VALUE_WIDTH),
            size.height,
        ));
        Ok(Self {
            panel,
            target,
            widgets,
            seen: controls.revision(),
        })
    }

    /// Show the panel with its top-left corner at `top_left` in AppKit screen points
    /// (origin bottom-left), or at the top right of the main screen when `None`, clear of a
    /// game window opened at its default centred position.
    pub fn show(&self, top_left: Option<[f64; 2]>) {
        let corner = top_left.map(|[x, y]| NSPoint::new(x, y)).or_else(|| {
            let mtm = MainThreadMarker::new()?;
            let visible = NSScreen::mainScreen(mtm)?.visibleFrame();
            let width = self.panel.frame().size.width;
            Some(NSPoint::new(
                visible.origin.x + visible.size.width - width - 20.0,
                visible.origin.y + visible.size.height - 20.0,
            ))
        });
        match corner {
            Some(corner) => self.panel.setFrameTopLeftPoint(corner),
            None => self.panel.center(),
        }
        self.panel.orderFrontRegardless();
    }

    /// Apply queued widget edits to `controls` (snapped, clamped and counted by the model),
    /// then refresh the widgets. Returns how many edits changed a value.
    pub fn apply_edits(&mut self, controls: &mut Controls) -> usize {
        let edits = std::mem::take(&mut *self.target.ivars().edits.borrow_mut());
        let changed = edits
            .into_iter()
            .filter(|edit| match edit {
                Edit::Number(id, shown) => controls.edit_display(*id, *shown),
                Edit::Value(id, value) => controls.edit_value(*id, value.clone()),
            })
            .count();
        self.sync(controls);
        changed
    }

    /// Move one widget as a person would and send its `AppKit` action, so tests and agents
    /// exercise the real target/action path. Sliders take a display value, toggles nonzero
    /// for on. Returns `false` for a readout, a text field or an unknown id.
    pub fn simulate(&self, id: ControlId, display_value: f64) -> bool {
        let control: &NSControl = match self.widgets.get(id.index()) {
            Some(Widget::Slider { slider, .. }) => {
                slider.setDoubleValue(display_value);
                slider
            }
            Some(Widget::Toggle(button)) => {
                button.setState(isize::from(display_value != 0.0));
                button
            }
            _ => return false,
        };
        #[expect(
            unsafe_code,
            reason = "sendAction:to: is the narrow AppKit action dispatch the panel uses"
        )]
        // SAFETY: the selector is the control's own `controlChanged:` action and the target
        // is this panel's PanelTarget, which implements it for an NSControl sender.
        unsafe {
            control.sendAction_to(control.action(), Some(&self.target));
        }
        true
    }

    /// Draw the panel offscreen, title bar and HUD background included, as PNG bytes. This
    /// works without a visible display (for example while the Mac is locked), so agents can
    /// review the panel they built.
    ///
    /// # Errors
    /// Returns a failure when the panel has no content view or `AppKit` cannot encode it.
    pub fn render_png(&self) -> Result<Vec<u8>, String> {
        let content = self
            .panel
            .contentView()
            .ok_or("the panel has no content view")?;
        content.layoutSubtreeIfNeeded();
        // The frame view draws the HUD material and title; the content view alone is
        // transparent with light text.
        #[expect(
            unsafe_code,
            reason = "AppKit marks superview unsafe because the hierarchy can change under it"
        )]
        // SAFETY: on the main thread with the panel alive, the frame view owns the content
        // view; the result is retained for the rest of this call.
        let frame = unsafe { content.superview() };
        let view = frame.unwrap_or(content);
        let bounds = view.bounds();
        let bitmap = view
            .bitmapImageRepForCachingDisplayInRect(bounds)
            .ok_or("AppKit could not allocate a bitmap for the panel")?;
        view.cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
        let properties = NSDictionary::new();
        #[expect(
            unsafe_code,
            reason = "AppKit's PNG encoder takes an untyped property dictionary"
        )]
        // SAFETY: the dictionary is empty, so no key or value can have the wrong type.
        let png = unsafe {
            bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
        };
        Ok(png
            .ok_or("AppKit could not encode the panel as PNG")?
            .to_vec())
    }

    /// Show values that changed in `controls` since this panel last looked, whether a
    /// person, a parameter file or the game changed them.
    pub fn sync(&mut self, controls: &Controls) {
        for id in controls.changed_since(self.seen) {
            let text = NSString::from_str(&controls.formatted(id));
            match (&self.widgets[id.index()], controls.value(id)) {
                (
                    Widget::Slider {
                        slider,
                        value,
                        scale,
                    },
                    Value::Number(number),
                ) => {
                    slider.setDoubleValue(f64::from(*number) * scale);
                    value.setStringValue(&text);
                }
                (Widget::Toggle(button), Value::Flag(on)) => button.setState(isize::from(*on)),
                (Widget::Text(field), Value::Text(_)) | (Widget::Readout(field), _) => {
                    field.setStringValue(&text);
                }
                _ => {}
            }
        }
        self.seen = controls.revision();
    }
}

impl Drop for MacPanel {
    fn drop(&mut self) {
        self.panel.close();
    }
}

fn heading(group: &str) -> String {
    let mut chars = group.chars();
    chars.next().map_or_else(String::new, |first| {
        first
            .to_uppercase()
            .chain(chars)
            .collect::<String>()
            .replace('_', " ")
    })
}

#[expect(
    unsafe_code,
    reason = "AppKit target/action wiring is the narrow Objective-C bridge for platform controls"
)]
fn build_row(
    label: &str,
    kind: &Kind,
    value: &Value,
    text: &str,
    id: ControlId,
    target: &PanelTarget,
    mtm: MainThreadMarker,
) -> (Retained<NSView>, Widget) {
    let name = NSTextField::labelWithString(&NSString::from_str(label), mtm);
    fixed_width(&name, LABEL_WIDTH);
    let tag = isize::try_from(id.index()).unwrap_or(isize::MAX);
    let target: &AnyObject = target;
    let action = Some(sel!(controlChanged:));
    let shown = NSString::from_str(text);
    let (views, widget): (Vec<Retained<NSView>>, Widget) = match (kind, value) {
        (
            Kind::Slider {
                min, max, display, ..
            },
            Value::Number(number),
        ) => {
            let scale = display.scale();
            // SAFETY: `target` is a live PanelTarget implementing `controlChanged:`; the panel
            // retains it for as long as the slider exists.
            let slider = unsafe {
                NSSlider::sliderWithValue_minValue_maxValue_target_action(
                    f64::from(*number) * scale,
                    f64::from(*min) * scale,
                    f64::from(*max) * scale,
                    Some(target),
                    action,
                    mtm,
                )
            };
            slider.setTag(tag);
            slider.setContinuous(true);
            fixed_width(&slider, SLIDER_WIDTH);
            let readout = NSTextField::labelWithString(&shown, mtm);
            readout.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                12.0, 0.0,
            )));
            fixed_width(&readout, VALUE_WIDTH);
            (
                vec![
                    name.into_super().into_super(),
                    slider.clone().into_super().into_super(),
                    readout.clone().into_super().into_super(),
                ],
                Widget::Slider {
                    slider,
                    value: readout,
                    scale,
                },
            )
        }
        (Kind::Toggle, Value::Flag(on)) => {
            // SAFETY: as above, the retained PanelTarget implements `controlChanged:`.
            let button = unsafe {
                NSButton::checkboxWithTitle_target_action(
                    &NSString::from_str(""),
                    Some(target),
                    action,
                    mtm,
                )
            };
            button.setTag(tag);
            button.setState(if *on { NSControlStateValueOn } else { 0 });
            (
                vec![
                    name.into_super().into_super(),
                    button.clone().into_super().into_super(),
                ],
                Widget::Toggle(button),
            )
        }
        (Kind::Text, _) => {
            let field = NSTextField::textFieldWithString(&shown, mtm);
            field.setTag(tag);
            // SAFETY: as above; text fields send their action when editing ends.
            unsafe { field.setTarget(Some(target)) };
            // SAFETY: `controlChanged:` is a valid selector implemented by the target.
            unsafe { field.setAction(action) };
            fixed_width(&field, SLIDER_WIDTH + VALUE_WIDTH);
            (
                vec![
                    name.into_super().into_super(),
                    field.clone().into_super().into_super(),
                ],
                Widget::Text(field),
            )
        }
        (Kind::Readout { .. } | Kind::Slider { .. } | Kind::Toggle, _) => {
            let readout = NSTextField::labelWithString(&shown, mtm);
            readout.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                12.0, 0.0,
            )));
            (
                vec![
                    name.into_super().into_super(),
                    readout.clone().into_super().into_super(),
                ],
                Widget::Readout(readout),
            )
        }
    };
    let row = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(&views), mtm);
    row.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    row.setSpacing(8.0);
    (row.into_super(), widget)
}

fn fixed_width(view: &NSView, width: f64) {
    view.widthAnchor()
        .constraintEqualToConstant(width)
        .setActive(true);
}
