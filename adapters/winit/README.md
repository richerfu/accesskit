# AccessKit winit adapter

This is the winit adapter for [AccessKit](https://accesskit.dev/). It exposes an AccessKit accessibility tree through the platform-native accessibility API on any platform supported by AccessKit. On platforms not supported by AccessKit, this adapter does nothing, but still compiles.

## Compatibility with async runtimes

The following only applies on Linux/Unix:

While this crate's API is purely blocking, it internally spawns asynchronous tasks on an executor.

- If you use tokio, make sure to enable the `tokio` feature of this crate.
- If you use another async runtime or if you don't use one at all, the default feature will suit your needs.

## Android activity compatibility

The Android implementation of this adapter currently only works with [GameActivity](https://developer.android.com/games/agdk/game-activity), which is one of the two activity implementations that winit currently supports.

## Examples

The `examples/` directory contains two runnable examples:

- `simple` — a minimal window exposing a single accessible label.
- `mixed_handlers` — demonstrates combining AccessKit's action handling with winit event handling.

On desktop platforms, run them with `cargo run --example simple` or `cargo run --example mixed_handlers` from this crate's directory.

### Running the examples on iOS

Install [XcodeGen](https://github.com/yonaskolb/XcodeGen) and the iOS Rust targets, then run `xcodegen` from `examples/apple/` to generate the Xcode project. Open it in Xcode and build/run the `Simple` or `MixedHandlers` target on a device or simulator.

## winit 0.31 and OpenHarmony

This branch targets the winit 0.31 beta revision pinned in the workspace manifest.
Windows and active event loops are trait objects. `Adapter::with_event_handler`
accepts a thread-safe callback instead of a typed user-event proxy. Queue each
AccessKit event and call `EventLoopProxy::wake_up`; drain the queue from
`ApplicationHandler::proxy_wake_up`. Both examples demonstrate this pattern.
Only raw-window-handle 0.6 is supported; the former `rwh_05` feature was removed.

On OpenHarmony, the adapter obtains the native accessibility provider from the
window's Ability XComponent. Drop the adapter during `destroy_surfaces`, before
destroying its window. The existing component is already visible, so the desktop
requirement to create the adapter before showing the window does not apply.
ArkUI supplies the component and window offsets; tree bounds remain in physical
surface coordinates. Rust 1.88 and OpenHarmony API 15 or later are required.
