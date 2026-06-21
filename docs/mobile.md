# Mobile build notes

DigitalRobbo targets Android and iOS via Bevy mobile templates.

## Android

1. Install Android SDK/NDK
2. Follow [Bevy Android setup](https://bevy.org/learn/quick-start/getting-started/setup/)
3. Build: `cargo apk build -p robbo-app`

## iOS

1. Xcode + Rust iOS targets
2. Follow Bevy iOS mobile guide
3. Build via Xcode project generated from template

Touch input: on-screen D-pad planned in `robbo-app/src/input.rs` (extend for mobile).

Persistence: use platform app storage implementing `SaveStorage` trait.
