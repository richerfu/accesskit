# accesskit_ohos

AccessKit adapter for the ArkUI native accessibility provider API on
OpenHarmony/HarmonyOS NEXT.

The adapter is attached to an accessibility `Provider` obtained from an
XComponent or an `ARKUI_NODE_CUSTOM`. It retains the AccessKit tree, answers
ArkUI's synchronous node queries, forwards native actions to AccessKit, and
reports tree/focus changes back to ArkUI.

```rust,ignore
let provider = xcomponent.accessibility_provider()?;
let adapter = accesskit_ohos::Adapter::new(
    provider,
    activation_handler,
    action_handler,
)?;

adapter.update_if_active(|| next_tree_update());
```

`Adapter::new` uses the standard callback registration recommended for a
single ArkUI custom node or XComponent. Frameworks that intentionally use
ArkUI's API 15 multi-instance callback contract can use
`Adapter::new_with_instance` instead.

The OpenHarmony native API does not expose an unregister function. Dropping
the adapter disables its Rust callback target, so any late native callback
returns a failure without touching released application state.

## License

MIT OR Apache-2.0
