# Aerugo

# Moved to codeberg

https://codeberg.org/i509vcb/aerugo

----

## What is Aerugo?

Aerugo is my attempt at a Wayland compositor where writing a window manager does not also require writing an
entire Wayland compositor.

I often see the question "where do I start to write a tiling wm in Wayland?" With Smithay and wlroots the answer is
pretty much "reinvent the wheel" and implement the entire display server along with the wm. My goal here is to provide an option
where only focus on the wm needs to happen.

## Features

To make the window manager easy to write, Aerugo provides a window management API boundary inside a wasm runtime.

A benefit to the window manager inside a wasm container is that the window manager can be run in the compositor
process for reduced latency and written in any programming language which can compile wasm bytecode with
Aerugo's WASM Interface Type (WIT) package.

Some key features *will* include:
- [ ] Programmable window manager through a wasm runtime
  - [ ] WM managed surfaces
- [ ] Runtime reloadable window management
- [ ] Support for standards such as XDG desktop portals
- [ ] Support for protocols such as layer shell

Eventually I will include a list of protocols somewhere in the repository.

## Getting started

Aerugo is still quite WIP: more coming soon!

## Licensing

Aerugo is released under the GNU General Public License v3.0.

The WASM interface type (WIT) definition for the Aerugo WM API is licensed under the MIT license.
