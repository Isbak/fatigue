# Safe and Fast Structural Fatigue Assessment as Code in Rust

[![Rust Security Audit](https://github.com/Isbak/fatigue/actions/workflows/security_audit.yml/badge.svg)](https://github.com/Isbak/fatigue/actions/workflows/security_audit.yml)

## Overview

This project aims to provide a reliable and efficient tool for conducting structural fatigue assessments, leveraging the Rust programming language's safety and performance features. Designed with the engineering community in mind, it offers a code-based approach to evaluate structural integrity under cyclic loading conditions, crucial for various industries including aerospace, automotive, construction, and more.

The implementation focuses on modern fatigue analysis methods and algorithms, ensuring high accuracy and computational efficiency. By utilizing Rust, we emphasize safety (both in terms of software reliability and engineering outcomes) and speed, catering to large-scale simulations and data-intensive computations.

## Features

- **Safety and Reliability**: Built in Rust to minimize common programming errors and ensure robust execution.
- **High Performance**: Optimized for speed, facilitating rapid assessments of complex structural models.
- **Modular Design**: Easy integration into existing engineering workflows or as part of a larger analysis pipeline.
- **Open Source**: Encouraging collaboration and contributions from the engineering and scientific community.

## Getting Started

### Prerequisites

- Rust toolchain (latest stable version recommended). If you don't have Rust installed, you can download it from [the official website](https://www.rust-lang.org/tools/install).
- Ensure you have Git installed to clone the repository. If not, download it from [Git's website](https://git-scm.com/downloads).

### Installation

1. Clone the repository:
```sh
  git clone https://github.com/Isbak/fatigue.git
```
2. Navigate to the project directory:
```sh
cd fatigue
```
3. Compile the project
```sh
cargo build --release
```

### Setting Up Rust for WebAssembly
Embark on the journey of WebAssembly development with Rust by following these steps to set up your environment:

Install Rust: If Rust isn't already your trusty sidekick, install it via [rustup](https://rustup.rs/), ensuring you have the latest stable version.

Add the WebAssembly Target: Unleash Rust's full potential by adding WebAssembly as a compilation target:

```sh
rustup target add wasm32-unknown-unknown
rustup target add wasmi
```
Install wasm-pack: To seamlessly pack your Rust code into WebAssembly, wasm-pack is the tool for the job, facilitating both compilation and packaging.

Optional Tools:

wasm-bindgen: For interacting between WebAssembly modules and JavaScript.
cargo-generate: To kickstart a Rust-WebAssembly project with a template.
web-sys: A crate providing bindings for Web APIs.
Compile Your Project: Navigate to your project directory and run:

```sh
wasm-pack build
```
Integration into Web Projects: Use the generated .wasm file along with the wasm-bindgen or web-sys crates to integrate Rust-powered functionality directly into your web applications.

This setup primes your Rust environment for diving into the vast sea of WebAssembly, ensuring your projects can ride the waves of the web with the power and safety of Rust.

```sh
cargo build --features wasm --target wasm32-wasi --release
```

## Usage
Here are some examples of how you can use the application or library:
```sh
fatigue -run yourconfig.yaml
```

## Contributing
We welcome contributions from the community! Please see our CONTRIBUTING.md file for guidelines on how to make contributions.

## License
This project is distributed under the MIT License. See the LICENSE file in the repository for more information.
