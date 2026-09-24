# Hydra RGB

Live lighting for the **Portronics Hydra 10**.

Pick a color, paint individual keys, save the look, and leave the app beside the clock. The keyboard’s own onboard effects stay untouched until you link it, and one click hands those effects back.

## Download for Windows

**[Download Hydra RGB for Windows](https://github.com/techistad/potronics-hydra10-rgbapp/raw/main/dist/Hydra-RGB-Windows.exe)**

The file is also in this repo at [`dist/Hydra-RGB-Windows.exe`](dist/Hydra-RGB-Windows.exe).

1. Plug the keyboard in with **USB-C**.
2. Run `Hydra-RGB-Windows.exe`.
3. Press **Link keyboard**.
4. Choose an effect, then **Save look**.

No install step. Windows may ask you to keep the file the first time you open it.

## What it does

| | |
|---|---|
| Effects | Solid, Gradient, Rainbow, Wave, Breathing, Reactive, Ripple, Per-key paint, Afterglow, Scan, Pulse, Sparkle |
| Custom looks | Five colors, arranged as Spread, Cycle, Blocks, or Chase |
| Per-key paint | Click a key on the on-screen keyboard to turn that light on or off |
| Afterglow | A press flares, spills onto nearby keys, then goes dark |
| Profiles | Saved on the PC. Load, delete, and pick which one Windows should open |
| Tray | The window **X** hides Hydra beside the clock and keeps the lights running |
| Quit | Right-click the red **H** and choose **Quit** to stop the app and return the keyboard to its onboard effects |
| Startup | Optional **Start with Windows**, with a chosen saved profile |

Rainbow uses its own spectrum. The color picker stays out of the way for any effect that does not need it.

## The cable that carries color

Live colors travel over **USB-C**.

Bluetooth and the 2.4 GHz dongle keep the keyboard on its original onboard effects. A look you save still lives on the computer. Plug the USB-C cable back in, open Hydra, and that look starts again.

**Reset to keyboard** stops the stream and leaves the factory effects in charge. The next launch stays that way until you press **Link keyboard**.

This app sends live color frames only. It does not rewrite the keyboard firmware.

## Build it yourself on Windows

Install [Rust](https://rustup.rs), then:

```bat
cd hydra-rs
cargo build --release
```

The program is `hydra-rs\target\release\hydra-rgb.exe`. `Run Hydra RGB Rust.bat` builds and opens it.

Saved profiles are written to `hydra-rs\profiles.json` on your machine. That file stays local.

## macOS build needed

There is no Mac download yet. This project was built on Windows, which cannot produce a macOS app.

If you have a Mac, the source is ready to compile:

1. Install [Rust](https://rustup.rs).
2. Install the Xcode command line tools (`xcode-select --install`).
3. From this folder, run:

```bash
cd hydra-rs
cargo build --release
./target/release/hydra-rgb
```

`dist/Hydra RGB.command` does those checks and then builds and opens the app.

**Please build the macOS version.** A pull request or a GitHub issue with an Apple Silicon build (and an Intel build, if you can make one) would give Mac owners the same download Windows already has. USB-C is the connection that can carry live color on macOS too.
