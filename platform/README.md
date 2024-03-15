# Platform-specific player realizations

## Rust + SwiftUI (iOS/iPadOS/macOS (iOS mode))

### Build `monolib`

```
cargo lipo --release --targets aarch64-apple-ios -p monolib
```

For running in simulator

```
cargo lipo --release --targets aarch64-apple-ios-sim,x86_64-apple-ios -p monolib
```

### Build and run app

Open Xcode and run.

[Screenshots (pre v0.2)](./screenshots/swiftui)
