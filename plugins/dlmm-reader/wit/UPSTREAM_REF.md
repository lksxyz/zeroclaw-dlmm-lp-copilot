# Pinned ZeroClaw registry WIT (`wit/v0`)

Copied verbatim from the official ZeroClaw plugin registry:

- Repo: https://github.com/Andy00L/zeroclaw-plugins
- Path: `wit/v0/`
- **UPSTREAM_REF: `23a5dcb953f697cae08d8e2802b39894ac9ddda1`**
- Pinned against ZeroClaw runtime **0.8.4** and `wit-bindgen` **0.46.0** (see
  `../Cargo.lock`).

Do not edit these files — if the registry moves, re-copy and bump UPSTREAM_REF.
The `tool-plugin` world in `tool.wit` (imports `logging`, exports
`plugin-info` + `tool`) is what `wit_bindgen::generate!` binds against.
