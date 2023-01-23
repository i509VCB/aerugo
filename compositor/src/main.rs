use clap::Parser;
use smithay::reexports::calloop::EventLoop;
use state::Aerugo;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod backend;
mod cli;
mod state;

fn main() {
    let args = cli::AerugoArgs::parse();

    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut r#loop = EventLoop::try_new_high_precision()
        .or_else(|_| {
            tracing::warn!("Failed to initialize high precision event loop, falling back to regular event loop");
            EventLoop::try_new()
        })
        .expect("Failed to create event loop");

    let mut state = Aerugo::new(&r#loop, &args).unwrap();
    r#loop
        .run(None, &mut state, |state| {
            state.check_shutdown();

            // Flush the display at the end of the idle callback to allow clients to process server events.
            state.flush_display();
        })
        .expect("Error while dispatching event loop");
}
