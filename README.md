# Draft

High-performance programmable markup compiler WIP

## Architecture

`draft-core` contains core utilities used to parse Draft markup and object notation. It

```mermaid
flowchart
    core[draft-core]
   core --> cli[draft-cli]
   core --> lsp[draft-lsp]
   lsp --> editor[draft-editor]
```

need for `rustfmt.toml`:
```bash
rustup toolchain install nightly
```