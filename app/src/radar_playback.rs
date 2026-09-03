//! Which frame of a radar run is on screen, and whether it is advancing.
//!
//! Split out of the bridge because playback is the one part of the radar
//! surface that answers to the viewer rather than to the network: the transport
//! buttons and the scrub handle drive these five entry points, and none of them
//! knows a run can be refetched. Keeping them together is what lets the module
//! beside this one stay about fetching a run and publishing it.
//!
//! Every one of them is a no-op on an empty run rather than an error. A window
//! whose first refresh has not landed still has a Play button, and a viewer
//! pressing it should get nothing, not a panic.

use std::pin::Pin;

use cxx_qt_lib::QString;

use crate::radar_controller::ffi;

impl ffi::RadarController {
    pub fn play(mut self: Pin<&mut Self>) {
        let can_play = *self.frame_count() > 1;
        self.as_mut().set_playing(can_play);
    }

    pub fn pause(mut self: Pin<&mut Self>) {
        self.as_mut().set_playing(false);
    }

    pub fn advance_frame(mut self: Pin<&mut Self>) {
        if !*self.playing() || *self.frame_count() == 0 {
            return;
        }
        let next = (*self.current_frame_index() + 1) % *self.frame_count();
        self.as_mut().set_current_frame_index(next);
        self.update_current_frame();
    }

    pub fn seek(mut self: Pin<&mut Self>, index: i32) {
        let last = self.frame_count().saturating_sub(1);
        if last < 0 {
            return;
        }
        self.as_mut().set_playing(false);
        self.as_mut().set_current_frame_index(index.clamp(0, last));
        self.update_current_frame();
    }

    pub(crate) fn update_current_frame(mut self: Pin<&mut Self>) {
        let index = isize::try_from(*self.current_frame_index()).unwrap_or_default();
        let label = self
            .timestamps()
            .get(index)
            .map_or_else(QString::default, Clone::clone);
        let url = format!(
            "image://radar/{}/{}",
            self.generation(),
            self.current_frame_index()
        );
        let observed = usize::try_from(index)
            .ok()
            .and_then(|index| self.observed.get(index).copied())
            .unwrap_or(true);
        self.as_mut().set_current_frame_observed(observed);
        self.as_mut().set_current_frame_label(label);
        self.set_current_frame_url(url.into());
    }
}
