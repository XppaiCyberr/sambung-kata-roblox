# Sambung Kata Overlay

Desktop helper for Indonesian `sambung kata`, built with `Tauri 2 + Svelte + TypeScript + Rust`.

## What It Does

- shows an always-on-top overlay window
- searches an Indonesian word list locally
- uses awalan-only search
- toggles the overlay with `Space`

## Run

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

## Notes

- The app avoids system-wide keystroke capture. It only uses a global shortcut to show or hide the overlay.
- The bundled dictionary currently merges `00-indonesian-wordlist.lst` and `01-kbbi3-2001-sort-alpha.lst` from `geovedi/indonesian-wordlist`, with duplicates removed at load time.
