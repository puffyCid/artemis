# Setup
1. See https://github.com/leptos-rs/leptos/tree/main/examples/tailwind_csr
2. Install DaisyUI `npm i -D daisyui@latest`
3. Install Typography `npm install -D @tailwindcss/typography`
4. Install trunk https://trunkrs.dev/
5. Compile the project to WASM. `trunk build --release` or start dev server `trunk serve --open`
6. WASM binary located: `../target/dist/web`
7. In `server` workspace run `cargo build --release --examples` to compile example server binary with WASM frontend!