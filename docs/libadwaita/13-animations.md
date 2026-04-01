# Adwaita Animations Reference

Libadwaita provides a complete animation framework with both duration-based and
physics-based (spring) animations. All animation classes live under the `adw`
namespace.

---

## AdwAnimation (Abstract Base)

The base class for all animations. Not instantiated directly -- use
`AdwTimedAnimation` or `AdwSpringAnimation`.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `widget` | `GtkWidget` | The widget this animation is associated with. |
| `target` | `AdwAnimationTarget` | The target that receives animation values. |
| `value` | `f64` | Current animation value (read-only). |
| `state` | `AdwAnimationState` | Current playback state (read-only). |
| `follow-enable-animations-setting` | `bool` | Whether to skip when system animations are disabled. Since 1.3. Default: `true`. |

### Methods

| Method | Description |
|--------|-------------|
| `play()` | Start or restart the animation. |
| `pause()` | Pause at current position. |
| `resume()` | Resume from paused state. |
| `reset()` | Reset to initial state without playing. |
| `skip()` | Jump immediately to the end value. |

### Signals

| Signal | Description |
|--------|-------------|
| `done` | Emitted when animation finishes (reaches end or is skipped). |

### Animation States

| State | Description |
|-------|-------------|
| `NotStarted` | Animation has not been played yet. |
| `Playing` | Animation is currently running. |
| `Paused` | Animation is paused mid-playback. |
| `Finished` | Animation has completed. |

### Behavior Notes

- Animations automatically skip if the widget is unmapped (not visible).
- Animations keep themselves alive while playing. You can create, start, and
  drop the reference immediately (fire-and-forget pattern).
- When `follow-enable-animations-setting` is `true` and system animations are
  disabled, the animation skips to the end value instantly.

---

## AdwTimedAnimation

Duration-based animation that interpolates between two values over a fixed time.

### Constructor

```rust
adw::TimedAnimation::new(
    widget: &impl IsA<gtk::Widget>,
    value_from: f64,
    value_to: f64,
    duration_ms: u32,
    target: &impl IsA<adw::AnimationTarget>,
) -> Self
```

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value-from` | `f64` | 0.0 | Starting value. |
| `value-to` | `f64` | 0.0 | Ending value. |
| `duration` | `u32` | 0 | Duration in milliseconds. |
| `easing` | `adw::Easing` | `EaseOutCubic` | Easing curve function. |
| `repeat-count` | `u32` | 1 | Number of times to repeat. 0 = infinite. |
| `reverse` | `bool` | `false` | Play from `value-to` to `value-from`. |
| `alternate` | `bool` | `false` | Reverse direction on alternating iterations. |

### Rust Example

```rust
use adw::prelude::*;

// Fade in a widget over 300ms
let target = adw::CallbackAnimationTarget::new(
    glib::clone!(@weak widget => move |value| {
        widget.set_opacity(value);
    }),
);

let animation = adw::TimedAnimation::new(&widget, 0.0, 1.0, 300, &target);
animation.set_easing(adw::Easing::EaseOutCubic);
animation.play();
```

### Repeating Animation

```rust
// Pulse animation that repeats 3 times, alternating direction
let target = adw::CallbackAnimationTarget::new(
    glib::clone!(@weak widget => move |value| {
        widget.set_opacity(0.5 + value * 0.5);
    }),
);

let animation = adw::TimedAnimation::new(&widget, 0.0, 1.0, 500, &target);
animation.set_repeat_count(3);
animation.set_alternate(true);
animation.play();
```

---

## AdwSpringAnimation

Physics-based animation using a spring model. Duration is calculated
automatically from the spring parameters. Produces natural-feeling motion with
optional overshoot.

### Constructor

```rust
adw::SpringAnimation::new(
    widget: &impl IsA<gtk::Widget>,
    value_from: f64,
    value_to: f64,
    spring_params: &adw::SpringParams,
    target: &impl IsA<adw::AnimationTarget>,
) -> Self
```

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `value-from` | `f64` | 0.0 | Starting value. |
| `value-to` | `f64` | 0.0 | Resting (target) position. |
| `spring-params` | `AdwSpringParams` | -- | Spring configuration (damping, mass, stiffness). |
| `initial-velocity` | `f64` | 0.0 | Starting velocity. Useful for gesture-driven animations. |
| `epsilon` | `f64` | 0.001 | Precision threshold. Animation stops when value is within epsilon of target. |
| `clamp` | `bool` | `false` | If `true`, stops immediately upon reaching `value-to` (no overshoot). |
| `estimated-duration` | `u32` | -- | Calculated duration in ms (read-only). |
| `velocity` | `f64` | -- | Current velocity (read-only). |

### Rust Example

```rust
use adw::prelude::*;

let spring = adw::SpringParams::new(0.8, 1.0, 100.0); // underdamped

let target = adw::CallbackAnimationTarget::new(
    glib::clone!(@weak widget => move |value| {
        widget.set_margin_start(value as i32);
    }),
);

let animation = adw::SpringAnimation::new(&widget, 0.0, 200.0, &spring, &target);
animation.play();
```

---

## AdwSpringParams

Describes the physical properties of a spring.

### Constructors

| Constructor | Parameters | Description |
|-------------|-----------|-------------|
| `SpringParams::new()` | `damping_ratio: f64, mass: f64, stiffness: f64` | Uses damping ratio (0-infinity). Recommended. |
| `SpringParams::new_full()` | `damping: f64, mass: f64, stiffness: f64` | Uses raw damping coefficient. For advanced use. |

### Damping Ratio Behavior

| Damping Ratio | Type | Behavior |
|---------------|------|----------|
| `0` | No damping | Oscillates endlessly, never settles. |
| `0 < r < 1` | Underdamped | Overshoots target, oscillates, then settles. Lower values = more bounce. |
| `1` | Critically damped | Reaches target as fast as possible without any overshoot. |
| `> 1` | Overdamped | Approaches target slowly without oscillation. Higher values = slower. |

### Common Presets

```rust
// Snappy, minimal overshoot
let snappy = adw::SpringParams::new(1.0, 1.0, 100.0);

// Bouncy, noticeable overshoot
let bouncy = adw::SpringParams::new(0.5, 1.0, 100.0);

// Gentle, slow approach
let gentle = adw::SpringParams::new(1.5, 1.0, 50.0);

// Stiff and fast
let stiff = adw::SpringParams::new(0.8, 0.5, 200.0);
```

---

## Animation Targets

Animation targets receive the interpolated value each frame and apply it.

### AdwCallbackAnimationTarget

Calls a custom closure each frame with the current value.

```rust
let target = adw::CallbackAnimationTarget::new(move |value| {
    // value is f64, use it however you need
    println!("Current value: {value}");
});
```

This is the most flexible target. Use it when you need to do anything beyond
setting a single property.

### AdwPropertyAnimationTarget

Directly animates a GObject property. No callback needed.

```rust
// Animate the "opacity" property of a widget from current to target
let target = adw::PropertyAnimationTarget::new(&widget, "opacity");
let animation = adw::TimedAnimation::new(&widget, 0.0, 1.0, 300, &target);
animation.play();
```

Only works with numeric properties (`f64`, `f32`, `i32`, `u32`). The property
must be writable.

### AdwNoneAnimationTarget (since 1.9)

A no-op target that discards the value. Use when you want the animation purely
as a frame clock or timer, reading the value from the `done` signal or polling
`animation.value()`.

```rust
let target = adw::NoneAnimationTarget::new();
let animation = adw::TimedAnimation::new(&widget, 0.0, 1.0, 1000, &target);

animation.connect_done(move |_| {
    // Fired after 1 second
    println!("Timer complete");
});

animation.play();
```

---

## Easing Functions

All 31 easing variants available in `adw::Easing`:

| Easing | Curve Type | Description |
|--------|-----------|-------------|
| `Linear` | -- | Constant speed, no acceleration. |
| `EaseInQuad` | Quadratic | Slow start, accelerating. |
| `EaseOutQuad` | Quadratic | Fast start, decelerating. |
| `EaseInOutQuad` | Quadratic | Slow start and end. |
| `EaseInCubic` | Cubic | Slow start, stronger acceleration. |
| `EaseOutCubic` | Cubic | Fast start, stronger deceleration. Default for timed animations. |
| `EaseInOutCubic` | Cubic | Smooth start and end. |
| `EaseInQuart` | Quartic | Very slow start. |
| `EaseOutQuart` | Quartic | Very fast start. |
| `EaseInOutQuart` | Quartic | Very smooth start and end. |
| `EaseInQuint` | Quintic | Extremely slow start. |
| `EaseOutQuint` | Quintic | Extremely fast start. |
| `EaseInOutQuint` | Quintic | Extremely smooth start and end. |
| `EaseInSine` | Sine | Gentle slow start. |
| `EaseOutSine` | Sine | Gentle fast start. |
| `EaseInOutSine` | Sine | Gentle smooth start and end. |
| `EaseInExpo` | Exponential | Near-zero start, explosive acceleration. |
| `EaseOutExpo` | Exponential | Explosive start, asymptotic end. |
| `EaseInOutExpo` | Exponential | Explosive middle. |
| `EaseInCirc` | Circular | Circular curve, slow start. |
| `EaseOutCirc` | Circular | Circular curve, fast start. |
| `EaseInOutCirc` | Circular | Circular curve, smooth. |
| `EaseInElastic` | Elastic | Wobbles at start. |
| `EaseOutElastic` | Elastic | Wobbles at end. |
| `EaseInOutElastic` | Elastic | Wobbles at start and end. |
| `EaseInBack` | Back | Pulls back before moving forward. |
| `EaseOutBack` | Back | Overshoots then returns. |
| `EaseInOutBack` | Back | Pulls back and overshoots. |
| `EaseInBounce` | Bounce | Bounces at start. |
| `EaseOutBounce` | Bounce | Bounces at end. |
| `EaseInOutBounce` | Bounce | Bounces at start and end. |

### Choosing an Easing

- **UI transitions**: `EaseOutCubic` (default) or `EaseOutQuad` for most cases.
- **Entrances**: `EaseOutCubic` or `EaseOutQuart` for elements appearing.
- **Exits**: `EaseInCubic` for elements disappearing.
- **Attention**: `EaseInOutCubic` for emphasis animations.
- **Playful UI**: `EaseOutBack` for slight overshoot, `EaseOutElastic` for bounce.
- **Mechanical motion**: `Linear` for progress bars or constant-rate updates.

---

## Fire-and-Forget Pattern

Animations keep themselves alive while playing. You do not need to store the
animation in a field unless you want to control it later (pause, reset, etc).

```rust
fn animate_entrance(widget: &gtk::Widget) {
    let target = adw::CallbackAnimationTarget::new(
        glib::clone!(@weak widget => move |value| {
            widget.set_opacity(value);
        }),
    );

    // No need to store this -- it stays alive until finished
    let animation = adw::TimedAnimation::new(widget, 0.0, 1.0, 250, &target);
    animation.play();
}
```

If you need to cancel or interact with the animation later, store it:

```rust
struct MyWidget {
    slide_animation: adw::TimedAnimation,
}

impl MyWidget {
    fn cancel_slide(&self) {
        self.slide_animation.skip();
    }
}
```

---

## Property Animation Example

Animate a widget property directly without a callback:

```rust
use adw::prelude::*;

fn fade_out(widget: &gtk::Widget) {
    let target = adw::PropertyAnimationTarget::new(widget, "opacity");
    let animation = adw::TimedAnimation::new(widget, 1.0, 0.0, 300, &target);
    animation.play();
}
```

---

## Spring Animation with Gesture Velocity

Pass gesture velocity to a spring animation for natural momentum:

```rust
use adw::prelude::*;

fn snap_to_position(widget: &gtk::Widget, velocity: f64) {
    let spring = adw::SpringParams::new(0.9, 1.0, 100.0);

    let target = adw::CallbackAnimationTarget::new(
        glib::clone!(@weak widget => move |value| {
            widget.set_margin_start(value as i32);
        }),
    );

    let animation = adw::SpringAnimation::new(
        widget,
        widget.margin_start() as f64,
        0.0,
        &spring,
        &target,
    );
    animation.set_initial_velocity(velocity);
    animation.play();
}
```

---

## Reduced Motion (since 1.9)

Most built-in libadwaita animations respect the `prefers-reduced-motion`
system preference. When enabled, sliding/scaling animations are replaced with
crossfades. For custom animations, check the setting:

```rust
let style_manager = adw::StyleManager::default();
// Or use CSS: @media (prefers-reduced-motion) { ... }
```

Set `follow-enable-animations-setting` to `false` on animations that should
always play regardless of user preference (e.g., progress indicators).
