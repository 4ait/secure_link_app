# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## service submodule update

git submodule update --remote --merge

## build app

$env:cargo tauri build --verbose ='-ext WixUtilExtension'
$env:TAURI_WIX_LIGHT_ARGS='-ext WixUtilExtension'

yarn tauri build --target x86_64-pc-windows-msvc --features windows 
.\tools\bundle.ps1
