# Contributing to Malgam

Thank you for your interest in the **Malgam** suite! We're glad to have you on the project.

Malgam is a high-resolution markup ecosystem built for performance and precision. We maintain high standards for architectural integrity and idiomatic Rust.

## 🛠️ Workspace Architecture

Malgam is structured as a Rust Workspace. Please ensure your contributions are placed in the correct crate:

- **`malgam-core`**: The engine. Contains parsing, lexing, and transformation logic.
- **`malgam-cli`**: The command-line interface (`mgc`).
- **`malgam-editor`**: The visual text editor interface.
- **`malgam-lsp`**: Language Server Protocol implementation.
- **`malgam-mgon`**: Malgam Object Notation handling.

## 🚀 Getting Started

1. **Fork the repository** and create your branch from `main`.
2. **Install Rust**: Ensure you are using the edition specified in the workspace `Cargo.toml`.
3. **Verify the Build**: Run `cargo build` from the root to ensure the workspace is healthy.

## 📝 Contribution Guidelines

### 1. Code Standards

- **Idiomatic Rust**: We prefer clear, safe, and idiomatic Rust. Use `clippy` to check for common improvements:
  ```bash
  cargo clippy --workspace -- -D warnings
  ```
- **Formatting**: All code must be formatted with rustfmt before submission:
  ```bash
  cargo fmt --all
  ```
- **Safety**: Avoid unsafe code unless strictly required for performance in malgam-core. All unsafe blocks must include a // SAFETY: comment.

### 2. Pull Request Process

- **Atomic Commits**: Keep your commits focused. One feature or one bug fix per PR is preferred.
- **The Prelude**: If you add new extension traits to core, ensure they are re-exported in malgam*core::prelude using the as * pattern to keep the API clean.
- **Testing**: Add unit tests for new logic. Run the full suite with cargo test.

### 3. Sustainable Development

We prioritize quality and developer longevity. We value deep-work sessions and thorough code reviews over rapid-fire iteration.

- **Review Latency**: Please allow the team up to 72 hours to respond to your PR or Issue.
- **Communication**: We encourage clear, asynchronous communication.

## ⚖️ License
By contributing to Malgam, you agree that your contributions will be licensed under the GNU Affero General Public License v3 (AGPL-3.0). This ensures the ecosystem remains free and open, even when provided over a network.
