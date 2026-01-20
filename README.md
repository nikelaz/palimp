# Palimp

![License: Proprietary](https://img.shields.io/badge/License-Source--Available-orange.svg)

**A high-performance, asynchronous Rust crawler engine with safety-first fetching and built-in archival.**

> [!WARNING]
> **Work in Progress:** Palimp is currently in active development. The API is unstable, and features are being added daily. Not yet suitable for any kind of use.

## Licensing & Usage

Palimp is a **Source Available** project. 

* **For Learners & Peers:** You are encouraged to inspect the code, study the architecture, and learn from the implementation.
* **For Users:** Any functional use of the software, whether for personal or commercial purposes, requires the purchase of a valid license.

This model allows me to keep the development transparent and the community informed while ensuring the project's sustainability. See the [LICENSE](LICENSE) file for the full legal text.

## Key Features (Current & Planned)

* **Safety-First Fetching:** * **Size Guards:** Aborts downloads if `Content-Length` exceeds safe limits (10MB default).
* **MIME Validation:** Strict `text/html` enforcement to prevent downloading binary bloat.
* **Redirect Tracking:** Automatic tracking of final URLs to prevent duplicate entries.
* **Asynchronous Engine:** Built on `Tokio` and `Reqwest` for high-concurrency without OS thread overhead.
* **Archival:** Integrated persistence layer to store page content, headers, and metadata.
* **Decoupled Architecture:** A standalone `core` designed to power both CLI and GUI interfaces.

## Project Structure

The planned architecture for Palimp is:

* `palimp-core`: The engine. Handles HTTP logic, safety checks, archival.
* `palimp-cli`: A terminal interface for running and monitoring crawls.
* `palimp-gui`: A desktop dashboard for visual crawling.

### How to use this

1. Create a file named `README.md` in your project root.
2. Paste the content above.
3. Since you just set **Neovim** as your git editor, you can commit this with:
`git add README.md`
`git commit -m "docs: add initial README with project vision"`

**Would you like me to help you write the `Cargo.toml` workspace file to actually link `palimp-core` and `palimp-cli` together?**
