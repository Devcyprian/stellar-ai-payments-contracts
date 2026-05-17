# Contributing

## Branch Model
- `main` — production releases
- `develop` — integration branch
- Feature branches: `feat/<scope>`, `fix/<scope>`

## Conventional Commits
```
feat(escrow): add partial release
fix(router): handle zero-amount edge case
```

## PR Checklist
- [ ] `cargo test --all` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] WASM builds: `cargo build --release --target wasm32-unknown-unknown`
- [ ] PR targets `develop`
