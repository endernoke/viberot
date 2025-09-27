# VibeRot Overlay Window

This Tauri app creates a translucent doomscrolling overlay window so you can embrace degradation whenever you want.

It can be used in conjunction with [VibeRot](https://github.com/endernoke/viberot) to autoplay brainrot while you wait for your commands to complete.

Actually it can load any URL, but transparency quality may vary depending on the platform. As of 2025-09-27, from testing, the translucent effect works well on Tiktok and is acceptable on Instagram and Reddit.

**This app is only tested on Windows**. If it works on other OSes, please open an issue to let me know.

## Building

Prerequisites:
- Npm
- Rust

```bash
npm run tauri build
```

## Usage

```bash
viberot-overlay [OPTIONS]
    -u, --url <URL>                URL to load in the overlay window [default: https://www.tiktok.com/foryou]
    -O, --opacity <OPACITY>        Opacity of the overlay window between 0.0 and 1.0 [default: 0.6]
    -h, --help                     Display this help message and exit
    -v, --version                  Display version information and exit
```

## License

This app is part of the [VibeRot](https://github.com/endernoke/viberot) project and is licensed under the MIT License. See the VibeRot repository for more details.