//! The window's fit across text sizes, screens, and the launch.
//!
//! It exercises only the public [`WindowFitter`] surface, as the theme bridge
//! does, against the app's own design sizes: an 852×672 minimum and a 980×760
//! opening size. [`sizing`] holds the rules that turn a text size and a screen
//! into the zoom and the two sizes; [`launch`] holds how the opening size
//! follows the screen while the window launches, and when it stops.

use crate::window_fit::{Screen, Size, WindowDesign, WindowFit, WindowFitter};

mod launch;
mod sizing;

const DESIGN: WindowDesign = WindowDesign {
    minimum: Size {
        width: 852,
        height: 672,
    },
    opening: Size {
        width: 980,
        height: 760,
    },
};

const fn size(width: u32, height: u32) -> Size {
    Size { width, height }
}

/// A monitor that Qt reports as wholly available, as Wayland does.
const fn monitor(width: u32, height: u32) -> Screen {
    Screen {
        size: size(width, height),
        available: size(width, height),
    }
}

fn fit_on(base_size: u32, screen: Option<Screen>) -> WindowFit {
    let mut fitter = WindowFitter::new(DESIGN, base_size);
    if let Some(screen) = screen {
        fitter.track_screen(screen);
    }
    fitter.fit()
}

fn fit(zoom: u32, minimum: Size, opening: Size) -> WindowFit {
    WindowFit {
        zoom,
        minimum,
        opening,
    }
}

/// Ends the launch the way the bridge does: a first frame, then two polls.
fn end_launch(fitter: &mut WindowFitter) {
    fitter.track_frame();
    assert!(
        !fitter.poll(),
        "the first poll after the frame ends nothing"
    );
    assert!(
        fitter.poll(),
        "the second poll after the frame ends the launch"
    );
}
