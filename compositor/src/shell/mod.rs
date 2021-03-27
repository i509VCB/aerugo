//! Wayland shell implementations
//!
//! This module provides implementations of the `layer` and `xdg shell` protocols.
//!
//! The primary way the compositor interacts with the shell is through the [self::Shell] object.

mod layer;
mod popup;
mod toplevel;

use std::error::Error;

use slog::Logger;
use smithay::{
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Display},
    utils::{Logical, Point, Rectangle, Size},
    wayland::{
        compositor::{self, is_sync_subsurface, with_surface_tree_upward, TraversalAction},
        shell::{
            wlr_layer::{self, LayerSurface},
            xdg::{PopupSurface, SurfaceCachedState, ToplevelSurface},
        },
    },
};

use crate::{
    shell::{popup::handle_popup_commit, toplevel::handle_toplevel_commit},
    state::State,
};

use self::toplevel::ToplevelInner;

/// The wayland shell.
///
/// The shell represents the active state of all displayed toplevel surfaces, popups and layers.
///
/// See the [Module documentation for more details](self).
#[derive(Debug)]
pub struct Shell {
    toplevels: Vec<Toplevel>,
    popups: Vec<Popup>,
    // TODO: Layers
}

impl Shell {
    pub fn new(_display: &mut Display, _logger: Logger) -> Result<Shell, Box<dyn Error>> {
        todo!("Implement the shell")
    }

    /// Inserts a new XDG shell window into the shell.
    pub fn new_xdg_toplevel(
        &mut self,
        toplevel: ToplevelSurface,
        position: Point<i32, Logical>,
    ) -> &mut Toplevel {
        let mut window = Toplevel {
            inner: ToplevelInner::Xdg(toplevel),
            size: Size::default(),
            position,
        };

        window.update();
        self.toplevels.insert(0, window);
        self.toplevels.first_mut().unwrap()
    }

    pub fn toplevels(&self) -> impl Iterator<Item = &Toplevel> {
        self.toplevels.iter()
    }

    pub fn toplevels_mut(&mut self) -> impl Iterator<Item = &mut Toplevel> {
        self.toplevels.iter_mut()
    }

    // TODO: Popup methods

    pub fn popups(&self) -> impl Iterator<Item = &Popup> {
        self.popups.iter()
    }

    pub fn popups_mut(&mut self) -> impl Iterator<Item = &mut Popup> {
        self.popups.iter_mut()
    }

    // TODO: Layer methods

    pub fn refresh(&mut self) {
        self.toplevels.retain(|w| w.inner.alive());
        self.popups.retain(|p| p.inner.alive());
        // TODO: Layers
    }

    /// Sends frame callbacks to the all surfaces.
    pub fn send_frames(&self, _time: u32) {
        todo!("Toplevels, Popups and Layers");
    }
}

/// A window.
#[derive(Debug)]
pub struct Toplevel {
    inner: ToplevelInner,
    size: Size<i32, Logical>,
    position: Point<i32, Logical>,
}

impl Toplevel {
    pub fn alive(&self) -> bool {
        self.inner.alive()
    }

    pub fn get_surface(&self) -> Option<&WlSurface> {
        self.inner.get_surface()
    }

    pub fn position(&self) -> Point<i32, Logical> {
        self.position
    }

    pub fn set_position(&mut self, new: Point<i32, Logical>) {
        self.position = new;
        self.update();
    }

    pub fn size(&self) -> Size<i32, Logical> {
        self.size
    }

    pub fn geometry(&self) -> Rectangle<i32, Logical> {
        // Generally the shell will be given the geometry by the client.
        compositor::with_states(self.inner.get_surface().unwrap(), |states| {
            states.cached_state.current::<SurfaceCachedState>().geometry
        })
        .unwrap()
        .unwrap_or_else(|| self.bbox()) // Fallback to bounding box where no geometry was given to us.
    }

    pub fn bbox(&self) -> Rectangle<i32, Logical> {
        Rectangle::from_loc_and_size(self.position, self.size)
    }

    pub fn send_configure(&self) {
        self.inner.send_configure()
    }

    fn update(&mut self) {
        todo!()
    }
}

#[derive(Debug)]
pub struct Popup {
    inner: PopupSurface,
}

impl Popup {
    pub fn alive(&self) -> bool {
        self.inner.alive()
    }

    pub fn get_surface(&self) -> Option<&WlSurface> {
        self.inner.get_surface()
    }

    pub fn send_configure(&self) {
        // This should never fail as the initial configure is always allowed.
        self.inner
            .send_configure()
            .expect("Popup initial configure should not fail");
    }

    // TODO: Parent

    // TODO: Position
}

#[derive(Debug)]
pub struct Layer {
    inner: LayerSurface,
    layer: wlr_layer::Layer,
}

impl State {
    pub fn handle_surface_commit(&mut self, surface: &WlSurface) {
        #[cfg(feature = "xwayland")]
        todo!("Commit hook");

        let shell = self.shell_mut();

        if !is_sync_subsurface(surface) {
            // Update buffer of all child surfaces
            with_surface_tree_upward(
                surface,
                (),
                |_, _, _| TraversalAction::DoChildren(()),
                |_, _states, _| todo!("Handle updating buffers of child surfaces"),
                |_, _, _| true,
            );
        }

        if let Some(toplevel) = shell
            .toplevels_mut()
            .find(|toplevel| toplevel.get_surface() == Some(surface))
        {
            handle_toplevel_commit(surface, toplevel);
        }

        if let Some(popup) = shell
            .popups_mut()
            .find(|popup| popup.get_surface() == Some(surface))
        {
            handle_popup_commit(surface, popup);
        }

        // TODO: Layers
    }
}
