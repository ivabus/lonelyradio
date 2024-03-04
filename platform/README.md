# Platform-specific player realizations

## Rust + SwiftUI (iOS/iPadOS/macOS (iOS mode))

### Build `monolib`

Run in `monolib` directory

```
cargo lipo --release --targets aarch64-apple-ios
```

For running in simulator

```
cargo lipo --release --targets aarch64-apple-ios-sim,x86_64-apple-ios
```

### Build and run app

Open Xcode and run.

[Screenshots](./screenshots/swiftui)
