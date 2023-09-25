//! A an example WM that only displays the last surface which was spawned.

use std::{collections::VecDeque, num::NonZeroU32};

use aerugo_wm::{Configure, States, ToplevelEvent, ToplevelId, ToplevelNode, Wm};
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};
use wayland_client::Connection;

struct WmState {
    toplevels: VecDeque<ToplevelId>,
    current: Option<ToplevelId>,
    node: Option<ToplevelNode>,
}

fn main() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env()
        .unwrap();
    let subscriber = FmtSubscriber::builder().with_env_filter(env_filter).finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let conn = Connection::connect_to_env().unwrap();
    tracing::info!("Connected to display");

    let mut wm = Wm::new(&conn).expect("Could not init wm");
    let mut state = WmState {
        toplevels: VecDeque::new(),
        current: None,
        node: None,
    };

    loop {
        wm.blocking_dispatch().expect("io error");

        while let Some(event) = wm.read_event() {
            match event {
                aerugo_wm::Event::Toplevel(event) => {
                    match event {
                        ToplevelEvent::New(toplevel) => {
                            println!("new toplevel: {toplevel:?}");
                        }
                        ToplevelEvent::Closed(toplevel) => {
                            println!("closed toplevel: {toplevel:?}");

                            // The WM may choose to do things like play an animation on close. For this
                            // example, just release the toplevel.
                            let _ = wm.release_toplevel(toplevel);
                        }
                    }
                }
            }
        }

        // Now that events have been processed, check if the current surface is the top surface.
        if state.toplevels.back() != state.current.as_ref() {
            state.current = state.toplevels.back().copied();
            // TODO: Handle destroy
            let _ = state.node.take();
        }

        let Some(current) = state.current else {
            // Nothing is mapped
            continue;
        };

        // Configure the current surface.
        let mut configure = Configure::default();
        configure.states(
                States::MAXIMIZED
                    | States::FULLSCREEN
                    | States::TILED_BOTTOM
                    | States::TILED_TOP
                    | States::TILED_LEFT
                    | States::TILED_RIGHT,
            )
            .size(Some((NonZeroU32::new(800).unwrap(), NonZeroU32::new(450).unwrap())));

        // Create a node to reference the toplevel.
        let node = wm.create_toplevel_node(current);

        // Create a transaction to apply the configure and present the node to the output.
        let mut transaction = wm.create_transaction();
        transaction.configure(current, configure);

        // Submit the transaction
    }
}
