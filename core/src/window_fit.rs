//! How large the window is drawn and sized at a given Omarchy text size.
//!
//! The window follows `omarchy-display-text-size` by zooming its whole content
//! by `base-size / 12`, so its layout stays the one that was tested at 12 px.
//! This module turns a base size and a screen into that zoom and into the two
//! window sizes that follow from it. It is whole-number arithmetic throughout:
//! the zoom is a count of [`ZOOM_UNIT`]ths, which makes `base-size / 12` exact
//! for every whole base size, and keeps floats, and the casts and comparisons
//! they bring, out of the Qt bridge that publishes it.
//!
//! The rules, decided with the owner on 2026-11-21:
//!
//! - **The zoom** is the base size over 12, capped so the zoomed minimum size
//!   fits the room the screen leaves. The cap limits only what the zoom adds:
//!   it never takes the zoom below 1, so at 12 px the window is exactly today's
//!   on any screen, also on one too small for it. Before any screen has been
//!   reported there is no cap.
//! - **The room** is the screen less Omarchy's bar, which on Wayland Qt does not
//!   subtract itself: it reports the whole monitor as available. The bar is 26
//!   px at 12 px and grows with the text size, as `[bar] scale-with-font` makes
//!   it by default. Where Qt does report a smaller available area, the room
//!   shrinks to that too.
//! - **The minimum size** is the design minimum zoomed and rounded up, so the
//!   layout inside is never handed less than its floor, and wherever the room
//!   holds the design minimum, the zoomed one fits the room.
//! - **The opening size** is the design opening size zoomed at the text size
//!   read at creation, and bounded to the room when the zoom is above 1. It is
//!   fitted when the window is created, and fitted again at each report of a
//!   different screen while the window is launching, because Qt names the
//!   primary screen until the window is mapped and the real monitor only
//!   after. Each fitting raises it to the minimum of that moment on either
//!   axis, since the window is resized to whatever the opening size says and
//!   Qt applies a size below the minimum as readily as any other. Nothing else
//!   moves it: a text size read during the launch changes only the zoom and the
//!   minimum, as one read later does, so a window that already fits keeps its
//!   size, one the person enlarged included, and Qt grows one that does not to
//!   the new minimum. The launch ends at the second poll after the window's
//!   first frame, at least one full poll interval after the compositor had
//!   that frame, and from then on no screen moves the opening size either, so
//!   an open window grows when it must and is otherwise left alone.

use crate::text_size::DEFAULT_BASE_SIZE;

/// The zoom's unit: a zoom of `ZOOM_UNIT` is 1, the window as designed.
pub const ZOOM_UNIT: u32 = 12_000;

/// The height of Omarchy's default horizontal bar at the default base size:
/// `[bar] size-horizontal` in the shell's generated `shell.toml`.
const BAR_HEIGHT: u32 = 26;

/// The polls after the first frame at which the launch ends.
const LAUNCH_POLLS: u8 = 2;

/// A width and a height, in logical pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// A screen as QML reports it: its size, and the available area Qt reports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Screen {
    pub size: Size,
    pub available: Size,
}

impl Screen {
    /// The screen a QML report describes, unless any number in it is not
    /// positive, as while Qt has not yet attached a window to a screen.
    #[must_use]
    pub fn from_report(
        width: i32,
        height: i32,
        available_width: i32,
        available_height: i32,
    ) -> Option<Self> {
        let side = |value: i32| u32::try_from(value).ok().filter(|side| *side > 0);
        Some(Self {
            size: Size {
                width: side(width)?,
                height: side(height)?,
            },
            available: Size {
                width: side(available_width)?,
                height: side(available_height)?,
            },
        })
    }
}

/// The window's sizes at a zoom of 1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowDesign {
    pub minimum: Size,
    pub opening: Size,
}

/// What the window is drawn and sized at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowFit {
    /// The content's zoom, in [`ZOOM_UNIT`]ths.
    pub zoom: u32,
    pub minimum: Size,
    pub opening: Size,
}

/// Keeps the inputs the window's fit follows, the opening size, and the
/// launch.
#[derive(Clone, Debug)]
pub struct WindowFitter {
    design: WindowDesign,
    base_size: u32,
    launch_base_size: u32,
    screen: Option<Screen>,
    opening: Size,
    launch: Launch,
}

#[derive(Clone, Copy, Debug)]
enum Launch {
    /// No frame shown yet.
    Starting,
    /// A frame has been shown, and this many polls have run since.
    Shown(u8),
    /// The launch is over, and the opening size is fixed.
    Ended,
}

impl WindowFitter {
    /// Starts a launch at `base_size`, the text size read at creation, which
    /// the opening size keeps for good.
    #[must_use]
    pub fn new(design: WindowDesign, base_size: u32) -> Self {
        Self {
            design,
            base_size,
            launch_base_size: base_size,
            screen: None,
            opening: launch_opening(design, base_size, base_size, None),
            launch: Launch::Starting,
        }
    }

    /// The fit for the current inputs.
    #[must_use]
    pub fn fit(&self) -> WindowFit {
        let zoom = zoom(self.design, self.base_size, self.screen);
        WindowFit {
            zoom,
            minimum: scaled_up(self.design.minimum, zoom),
            opening: self.opening,
        }
    }

    /// Whether the window is still launching, so a screen can move the
    /// opening size.
    #[must_use]
    pub const fn launching(&self) -> bool {
        !matches!(self.launch, Launch::Ended)
    }

    /// Takes a new text size, returning whether the fit changed. The zoom and
    /// the minimum follow it; the opening size never does.
    pub fn set_base_size(&mut self, base_size: u32) -> bool {
        let before = self.fit();
        self.base_size = base_size;
        self.fit() != before
    }

    /// Takes the screen the window is on, returning whether the fit changed.
    /// While the window is launching, a screen other than the last one
    /// reported fits the opening size again; a report of the same screen
    /// fits nothing again.
    pub fn track_screen(&mut self, screen: Screen) -> bool {
        let before = self.fit();
        let moved = self.screen != Some(screen);
        self.screen = Some(screen);
        if moved && self.launching() {
            self.opening = launch_opening(
                self.design,
                self.launch_base_size,
                self.base_size,
                self.screen,
            );
        }
        self.fit() != before
    }

    /// Notes that the window has shown a frame.
    pub const fn track_frame(&mut self) {
        if let Launch::Starting = self.launch {
            self.launch = Launch::Shown(0);
        }
    }

    /// Counts one poll, returning whether it ended the launch.
    pub fn poll(&mut self) -> bool {
        let Launch::Shown(polls) = self.launch else {
            return false;
        };
        let polls = polls.saturating_add(1);
        self.launch = if polls < LAUNCH_POLLS {
            Launch::Shown(polls)
        } else {
            Launch::Ended
        };
        !self.launching()
    }
}

/// The zoom `base_size` asks for, capped by `screen` when there is one.
fn zoom(design: WindowDesign, base_size: u32, screen: Option<Screen>) -> u32 {
    let requested = base_size.saturating_mul(ZOOM_UNIT / DEFAULT_BASE_SIZE);
    screen.map_or(requested, |screen| {
        requested.min(fit(design.minimum, room(screen, base_size)).max(ZOOM_UNIT))
    })
}

/// The opening size a fitting during the launch gives: the design opening size
/// zoomed at `launch_base_size`, the text size read at creation, raised to the
/// minimum at `base_size`, the one read last.
fn launch_opening(
    design: WindowDesign,
    launch_base_size: u32,
    base_size: u32,
    screen: Option<Screen>,
) -> Size {
    at_least(
        opening(design, launch_base_size, screen),
        scaled_up(design.minimum, zoom(design, base_size, screen)),
    )
}

/// The design opening size, zoomed for `base_size` and bounded to the room.
fn opening(design: WindowDesign, base_size: u32, screen: Option<Screen>) -> Size {
    let zoom = zoom(design, base_size, screen);
    let opening = rounded(design.opening, zoom);
    match screen {
        Some(screen) if zoom > ZOOM_UNIT => {
            let room = room(screen, base_size);
            Size {
                width: opening.width.min(room.width),
                height: opening.height.min(room.height),
            }
        }
        _ => opening,
    }
}

/// `size`, raised to `floor` on each axis where it falls short of it.
fn at_least(size: Size, floor: Size) -> Size {
    Size {
        width: size.width.max(floor.width),
        height: size.height.max(floor.height),
    }
}

/// What `screen` leaves the window: less the bar, and within the available area.
fn room(screen: Screen, base_size: u32) -> Size {
    Size {
        width: screen.size.width.min(screen.available.width),
        height: screen
            .size
            .height
            .saturating_sub(bar_height(base_size))
            .min(screen.available.height),
    }
}

/// Omarchy's bar at `base_size`: `round(26 × base-size / 12)`, as the shell's
/// `barToken` scales it.
fn bar_height(base_size: u32) -> u32 {
    let scaled = u64::from(BAR_HEIGHT) * u64::from(base_size);
    let default = u64::from(DEFAULT_BASE_SIZE);
    saturate((scaled + default / 2) / default)
}

/// The largest zoom at which `minimum`, rounded up, still fits `room`.
fn fit(minimum: Size, room: Size) -> u32 {
    let axis = |room: u32, minimum: u32| {
        saturate(u64::from(room) * u64::from(ZOOM_UNIT) / u64::from(minimum.max(1)))
    };
    axis(room.width, minimum.width).min(axis(room.height, minimum.height))
}

/// `design` zoomed by `zoom`, each side rounded up.
fn scaled_up(design: Size, zoom: u32) -> Size {
    let zoomed = |length: u32| {
        saturate((u64::from(length) * u64::from(zoom)).div_ceil(u64::from(ZOOM_UNIT)))
    };
    Size {
        width: zoomed(design.width),
        height: zoomed(design.height),
    }
}

/// `design` zoomed by `zoom`, each side rounded to the nearest pixel.
fn rounded(design: Size, zoom: u32) -> Size {
    let unit = u64::from(ZOOM_UNIT);
    let zoomed = |length: u32| saturate((u64::from(length) * u64::from(zoom) + unit / 2) / unit);
    Size {
        width: zoomed(design.width),
        height: zoomed(design.height),
    }
}

fn saturate(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
