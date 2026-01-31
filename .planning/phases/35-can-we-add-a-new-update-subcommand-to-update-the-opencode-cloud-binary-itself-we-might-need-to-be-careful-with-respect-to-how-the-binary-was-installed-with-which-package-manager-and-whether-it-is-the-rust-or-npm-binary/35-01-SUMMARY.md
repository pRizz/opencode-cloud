Summary:
- Added `occ update cli` subcommand to update the opencode-cloud binary using detected install method (cargo or npm) and restart the service afterward.
- Documented the new `update cli` and `update container` commands in the root README (syncs to `packages/core/README.md` via hook).

Tests:
- `just fmt`
- `just lint`
- `just test`
- `just build`
