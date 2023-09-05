# Aerugo compositor

## Protocol support

Alongside the core Wayland protocols, below is listed the extension protocols that are implemented or are
planned to be implemented:

<!-- Sort in the following order:
- xdg
- wp
- xwayland
- ext
- wlr
- others
-->

| Protocol                | Version/Supported | Notes   |
|-------------------------|-------------------|---------|
| XDG Shell               | TODO              |         | <!-- xdg -->
| XDG Decoration          | ❌                 | Planned |
| XDG Output              | ❌                 | Planned |
| XDG Activation          | ❌                 | Planned |
| Viewporter              | ❌                 | Planned | <!-- wp -->
| DRM lease               | ❌                 | Planned |
| Linux Dmabuf            | 4                 |         |
| Input method            | ❌                 | Planned |
| Single Pixel Buffer     | ❌                 | Planned |
| Content type hint       | ❌                 | Planned |
| Tearing control         | ❌                 | Planned |
| Fractional scale        | ❌                 | Planned |
| Cursor shape            | ❌                 | Planned |
| Security context        | ❌                 | Planned; only advertised to privileged clients |
| Idle inhibit            | ❌                 | Planned |
| Pointer constraints     | ❌                 | Planned |
| Primary selection       | ❌                 | Planned |
| Tablet                  | ❌                 | Planned |
| Xwayland shell          | ❌                 | Planned; Smithay needs to implement | <!-- xwayland -->
| Session Lock            | ❌                 | Planned; only advertised to privileged clients | <!-- ext -->
| Foreign toplevel list   | 1                 | Only advertised to privileged clients |
| Layer Shell             | ❌                 | Planned when released |
| WLR Layer Shell         | ❌                 | Planned | <!-- wlr -->
| WLR Output Management   | ❌                 | Planned |
| Aerugo Shell            | 1                 | Only advertised to privileged clients | <!-- others -->  
