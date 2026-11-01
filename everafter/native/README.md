# Everafter native apps

The Tauri 2 shell opens the complete planner in its own application window. It embeds the same Rust/Axum application used by the browser prototype; it does not require a running development server or tunnel. Frontend files are embedded into the executable by the shared Rust library. SQLite lives in the platform's private application data directory.

## What is shared

Dashboard, wedding-style starter checklists, categories/tiers, guests, local contact-file import and category suggestions, RSVP forms, rooms, transport assignments, vendors, budgets, and prompt-based invitation generation all use the same source. Internet is required only for external services such as paid image generation and optional fonts.

The app is local-first. Each device has its own database. Cross-device synchronization and remote guest links need a separately hosted service and account/permission implementation; they are not supplied by this native wrapper. A loopback guest link is useful on the same device only.

## Windows

Double-click `outputs/Everafter.exe` when provided. No server console needs to remain open. Microsoft Edge WebView2 Runtime must be installed (it is already present on the development machine). This development executable is unsigned.

The first launch looks for `everafter/everafter.db` beside the executable and copies it through SQLite's online backup API into the app data directory. Existing app-owned data is never overwritten. Moving the executable alone does not move its database; the app retains data in its private application directory.

Build from this folder using `cargo build --bin everafter-desktop`, or run `Build-Windows.ps1`. For a release executable use `cargo build --release --bin everafter-desktop`. Install the Tauri CLI with `cargo install tauri-cli --version '^2' --locked`, then use `cargo tauri build --bundles nsis` to make a Windows installer after verifying the executable.

## Android (requires Android build tools)

Install Android Studio, its SDK, platform tools, NDK, and Java according to the official prerequisites. Set `JAVA_HOME`, `ANDROID_HOME`, and `NDK_HOME`. Add the Android Rust targets with `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`.

From this folder:

```text
cargo install tauri-cli --version '^2' --locked
cargo tauri android init
```

Run `node configure-android.mjs` after initialization to install the supplied network-security configuration allowing HTTP only to loopback. Then run:

```text
cargo tauri android dev
cargo tauri android build --apk
```

The generated project appears under `gen/android`. Follow Tauri's signing guide for distributable APKs. SDK/NDK/Java are not installed on the development machine, so an APK has not yet been compiled or device-tested.

## iPhone (requires macOS and Xcode)

Transfer this entire `everafter` source folder to a Mac, install Rust and Xcode, and add the iOS targets using the official prerequisites. Then, from this folder:

```text
cargo install tauri-cli --version '^2' --locked
cargo tauri ios init
cargo tauri ios dev
cargo tauri ios build
```

Choose your signing team in Xcode. `Info.plist` permits the embedded server's local networking. No IPA or iPhone build has been produced on Windows; iOS compilation, signing, simulator testing, and physical-device testing remain required.

## Native-feature verification remaining

The app shell reuses the existing browser interface. Contact Picker access depends on the system WebView and is not guaranteed; CSV/vCard import is the implemented fallback. A custom native address-book permission/picker integration is not implemented. Print/PDF, file downloads, external links, and mobile file upload behavior need device-specific verification before mobile distribution. These should not be assumed tested merely because shared screens compile.

## Official references

- [Tauri prerequisites for Windows, Android, and iOS](https://v2.tauri.app/start/prerequisites/)
- [Tauri mobile entry point](https://v2.tauri.app/develop/calling-rust/)
- [Tauri configuration](https://v2.tauri.app/reference/config/)
- [Android signing](https://v2.tauri.app/distribute/sign/android/)
- [iOS signing](https://v2.tauri.app/distribute/sign/ios/)
