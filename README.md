# KOSMOS Explorer

Desktop-Client für OpenCloud — basierend auf Windows 11 UI (React) mit Tauri v2 (Rust).

## Architektur

- **Links:** Win11-Baumansicht (Clouds → Spaces → Ordner) — React
- **Rechts:** OpenCloud Web im Compact-Modus — natives WebView
- **Backend:** Tauri v2 (Rust) — OIDC, Token-Verwaltung, API-Proxy

## Features

- Multi-Cloud: mehrere OpenCloud-Instanzen verwalten
- OIDC-Login mit PKCE (OpenCloudDesktop Client)
- Spaces + Ordner-Navigation im Win11-Baumstil
- Dateien öffnen in separaten Fenstern (Collabora, PDF, Text)
- Win11 Platform Theme für OpenCloud Web

## Stack

- Frontend: React + Redux (Fork von [Win11web](https://github.com/AStrek016/Win11web))
- Desktop: Tauri v2 (Rust, WebKit/WebView2)
- Styling: SCSS + Tailwind (Win11 Theme)
- API: OpenCloud Graph API + WebDAV

## Build

```bash
npm install
npm run tauri build
```

Binaries landen unter `src-tauri/target/release/bundle/`.

## Entwicklung

```bash
npm run tauri dev
```

## Lizenz

CC0-1.0 (Frontend), MIT (Tauri/Rust)

---

Ein Projekt von [KOSMOS](https://codeberg.org/kosmos-lab).
