error_chain!{}

#[cfg(all(unix, not(target_os = "android")))]
mod platform {
    extern crate simple_signal;

    use self::simple_signal::Signal;
    use super::Result;

    pub fn set_shutdown_signal_handler<F>(f: F) -> Result<()>
    where
        F: Fn() + 'static + Send,
    {
        simple_signal::set_handler(&[Signal::Term, Signal::Int], move |s| {
            debug!("Process received signal: {:?}", s);
            f();
        });
        Ok(())
    }
}

#[cfg(windows)]
mod platform {
    extern crate ctrlc;

    use super::{Result, ResultExt};

    pub fn set_shutdown_signal_handler<F>(f: F) -> Result<()>
    where
        F: Fn() + 'static + Send,
    {
        ctrlc::set_handler(move || {
            debug!("Process received Ctrl-c");
            f();
        }).chain_err(|| "Unable to attach ctrl-c handler")
    }
}

#[cfg(target_os = "android")]
mod platform {
    use super::Result;

    pub fn set_shutdown_signal_handler<F>(_: F) -> Result<()>
    where
        F: Fn() + 'static + Send,
    {
        warn!("No shutdown signal handler was registered");
        Ok(())
    }
}

pub use self::platform::*;
