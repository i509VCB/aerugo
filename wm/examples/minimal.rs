use aerugo_wm::{ToplevelEvent, Wm};
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};
use wayland_client::Connection;

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
    }
}
