use std::{collections::HashMap, num::NonZeroU64, process::ExitCode};

use aerugo_wm::protocol::{
    aerugo_wm_toplevel_v1::{self, AerugoWmToplevelV1},
    aerugo_wm_v1::AerugoWmV1,
};
use foreign_toplevel::protocol::{
    ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
};
use once_cell::unsync::OnceCell;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use wayland_client::{
    globals::{self, BindError, GlobalList, GlobalListContents},
    protocol::wl_registry::WlRegistry,
    Connection, Dispatch, Proxy,
};

mod aerugo_wm;
mod foreign_toplevel;

const FOREIGN_TOPLEVEL_LIST: u32 = 1;
const AERUGO_WM: u32 = 1;

fn main() -> ExitCode {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env()
        .unwrap();
    let subscriber = FmtSubscriber::builder().with_env_filter(env_filter).finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut queue) = globals::registry_queue_init(&conn).unwrap();

    let aerugo_wm_v1 = globals.bind::<AerugoWmV1, _, _>(&queue.handle(), AERUGO_WM..=AERUGO_WM, ());
    let foreign_toplevel_list = globals.bind::<ExtForeignToplevelListV1, _, _>(
        &queue.handle(),
        FOREIGN_TOPLEVEL_LIST..=FOREIGN_TOPLEVEL_LIST,
        (),
    );

    // Record missing globals
    let mut missing_globals = Vec::new();

    test_global(
        &globals,
        &mut missing_globals,
        &foreign_toplevel_list,
        FOREIGN_TOPLEVEL_LIST,
    );

    test_global(&globals, &mut missing_globals, &aerugo_wm_v1, AERUGO_WM);

    if !missing_globals.is_empty() {
        for missing in missing_globals.iter() {
            match missing {
                MissingGlobal::NotAvailable { interface } => {
                    tracing::error!("Required global \"{interface}\" not available");
                }
                MissingGlobal::IncompatibleVersion {
                    interface,
                    required_version,
                    available,
                } => {
                    tracing::error!(
                        required_version,
                        available,
                        "Compatible version of \"{interface}\" not available"
                    );
                }
            }
        }

        tracing::error!("Exiting due to missing required globals");
        tracing::error!("Help: the window management client may be running under an incompatible compositor");
        return ExitCode::FAILURE;
    }

    // The client would have exited by now if any globals were missing.
    let aerugo_wm_v1 = aerugo_wm_v1.unwrap();
    let _foreign_toplevel_list = foreign_toplevel_list.unwrap();

    let mut state = State {
        aerugo_wm_v1,
        toplevels: HashMap::new(),
    };

    loop {
        queue.blocking_dispatch(&mut state).expect("Dispatch error");
    }

    ExitCode::SUCCESS
}

fn test_global<I: Proxy>(
    globals: &GlobalList,
    missing: &mut Vec<MissingGlobal>,
    result: &Result<I, BindError>,
    required_version: u32,
) {
    if let Err(ref err) = result {
        let interface = I::interface().name;

        match err {
            BindError::UnsupportedVersion => {
                // Find the highest version global
                let available = globals
                    .contents()
                    .with_list(|globals| {
                        globals
                            .iter()
                            .filter(|global| global.interface == interface)
                            .max_by(|a, b| a.version.cmp(&b.version))
                            .map(|global| global.version)
                    })
                    .expect("If the version is unsupported, the global must be available at some version");

                missing.push(MissingGlobal::IncompatibleVersion {
                    interface,
                    required_version,
                    available,
                })
            }

            BindError::NotPresent => {
                missing.push(MissingGlobal::NotAvailable { interface });
            }
        }
    }
}

enum MissingGlobal {
    NotAvailable {
        interface: &'static str,
    },

    IncompatibleVersion {
        interface: &'static str,
        required_version: u32,
        available: u32,
    },
}

struct State {
    aerugo_wm_v1: AerugoWmV1,
    toplevels: HashMap<NonZeroU64, Toplevel>,
}

#[derive(Debug)]
struct Toplevel {
    identifier: OnceCell<String>,
    current: ToplevelInfo,
    pending: Option<ToplevelInfo>,
    handle: ExtForeignToplevelHandleV1,
    wm_toplevel: AerugoWmToplevelV1,
}

impl Toplevel {
    /// Get or create the pending state.
    fn pending(&mut self) -> &mut ToplevelInfo {
        self.pending.get_or_insert_with(|| self.current.clone())
    }
}

#[derive(Debug, Clone)]
struct ToplevelInfo {
    app_id: Option<String>,
    title: Option<String>,
    capabilities: Vec<aerugo_wm_toplevel_v1::Capabilities>,
    /// width, height
    min_size: Option<(i32, i32)>,
    max_size: Option<(i32, i32)>,
    /// Id of parent.
    parent: Option<NonZeroU64>,
    /// x, y, width, length
    geometry: Option<(i32, i32, i32, i32)>,
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
