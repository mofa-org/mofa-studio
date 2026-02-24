# mofa-dora-bridge examples

## Runtime backend demo (Task 15 Phase 1)

This example demonstrates runtime backend selection for Studio dataflow lifecycle:

- `dora-cli` (current default behavior)
- `mofa-native` (reserved for Task 15 follow-up implementation)

### Dry run (parsing + backend selection)

```bash
cargo run -p mofa-dora-bridge --example runtime_backend_demo -- apps/mofa-fm/dataflow/voice-chat.yml
```

### Start/stop lifecycle with Dora backend

```bash
MOFA_RUNTIME_BACKEND=dora-cli \
cargo run -p mofa-dora-bridge --example runtime_backend_demo -- apps/mofa-fm/dataflow/voice-chat.yml --start
```

### Verify explicit unsupported behavior for mofa-native (phase 1)

```bash
MOFA_RUNTIME_BACKEND=mofa-native \
cargo run -p mofa-dora-bridge --example runtime_backend_demo -- apps/mofa-fm/dataflow/voice-chat.yml --start
```

Expected output includes an explicit `Unsupported runtime backend` error message.

