# Aerugo

## What is Aerugo?

Aerugo is my attempt at a Wayland compositor where writing a window manager does not also require writing an
entire Wayland compositor.

## Features

To make the window manager easy to write, Aerugo provides a window management API boundary inside a wasm runtime.

A benefit to the window manager inside a wasm container is that the window manager can be run in the compositor
process for reduced latency and written in any programming language which can compile wasm bytecode with
Aerugo's WASM Interface Type (WIT) package.

Some key features *will* include:
- [ ] Programmable window manager through a wasm runtime
- [ ] Runtime reloadable window management
- [ ] Support for standards such as XDG desktop portals
- [ ] Support for protocols such as layer shell

Eventually I will include a list of protocols somewhere in the repository.

## Getting started

Aerugo is still quite WIP: more coming soon!
