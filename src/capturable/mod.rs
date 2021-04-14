pub mod autopilot;
use std::boxed::Box;
use std::error::Error;
use tracing::warn;

#[cfg(target_os = "linux")]
pub mod pipewire;
#[cfg(target_os = "linux")]
pub mod pipewire_dbus;
pub mod testsrc;
#[cfg(target_os = "linux")]
pub mod x11;

pub trait Recorder {
    fn capture(&mut self) -> Result<crate::video::PixelProvider, Box<dyn Error>>;
}

pub trait BoxCloneCapturable {
    fn box_clone(&self) -> Box<dyn Capturable>;
}

impl<T> BoxCloneCapturable for T
where
    T: Clone + Capturable + 'static,
{
    fn box_clone(&self) -> Box<dyn Capturable> {
        Box::new(self.clone())
    }
}

pub trait Capturable: Send + BoxCloneCapturable {
    /// Name of the Capturable, for example the window title, if it is a window.
    fn name(&self) -> String;

    /// Return x, y, width, height of the Capturable as floats relative to the absolute size of the
    /// screen. For example x=0.5, y=0.0, width=0.5, height=1.0 means the right half of the screen.
    fn geometry_relative(&self) -> Result<(f64, f64, f64, f64), Box<dyn Error>>;

    /// Callback that is called right before input is simulated.
    /// Useful to focus the window on input.
    fn before_input(&mut self) -> Result<(), Box<dyn Error>>;

    /// Return a Recorder that can record the current capturable.
    fn recorder(&self, capture_cursor: bool) -> Result<Box<dyn Recorder>, Box<dyn Error>>;
}

impl Clone for Box<dyn Capturable> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

pub fn get_capturables(
    #[cfg(target_os = "linux")] wayland_support: bool,
    #[cfg(target_os = "linux")] capture_cursor: bool,
) -> Vec<Box<dyn Capturable>> {
    let mut capturables: Vec<Box<dyn Capturable>> = vec![];
    #[cfg(target_os = "linux")]
    {
        if wayland_support {
            use crate::capturable::pipewire::get_capturables as get_capturables_pw;
            match get_capturables_pw(capture_cursor) {
                Ok(captrs) => {
                    for c in captrs {
                        capturables.push(Box::new(c));
                    }
                }
                Err(err) => warn!(
                    "Failed to get list of capturables via dbus/pipewire: {}",
                    err
                ),
            }
        }

        use crate::capturable::x11::X11Context;
        let x11ctx = X11Context::new();
        if let Some(mut x11ctx) = x11ctx {
            match x11ctx.capturables() {
                Ok(captrs) => {
                    for c in captrs {
                        capturables.push(Box::new(c));
                    }
                }
                Err(err) => warn!("Failed to get list of capturables via X11: {}", err),
            }
        };
    }
    use crate::capturable::autopilot::AutoPilotCapturable;
    capturables.push(Box::new(AutoPilotCapturable::new()));

    if crate::log::get_log_level() >= tracing::Level::DEBUG {
        for (width, height) in [
            (200, 200),
            (800, 600),
            (1080, 720),
            (1920, 1080),
            (3840, 2160),
            (15360, 2160),
        ]
        .iter()
        {
            capturables.push(Box::new(testsrc::TestCapturable {
                width: *width,
                height: *height,
            }));
        }
    }

    capturables
}
