use chrono;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Output,
};
use log;
use std::{fmt, io, path::PathBuf};
use talpid_core::logging::rotate_log;

#[derive(err_derive::Error, Debug)]
pub enum Error {
    /// Unable to open log file for writing
    #[error(display = "Unable to open log file for writing: {}", path)]
    WriteFile {
        path: String,
        #[error(cause)]
        source: io::Error,
    },

    #[error(display = "Unable to rotate daemon log file")]
    RotateLog(#[error(cause)] talpid_core::logging::RotateLogError),

    #[error(display = "Unable to set logger")]
    SetLoggerError(#[error(cause)] log::SetLoggerError),
}

const SILENCED_CRATES: &[&str] = &[
    "jsonrpc_core",
    // jsonrpc_core does some logging under the "rpc" target as well.
    "rpc",
    "tokio_core",
    "tokio_io",
    "tokio_proto",
    "tokio_reactor",
    "tokio_threadpool",
    "jsonrpc_ws_server",
    "want",
    "ws",
    "mio",
    "hyper",
    "rtnetlink",
    "netlink_proto",
    "netlink_sys",
    "iproute2",
];
const SLIGHTLY_SILENCED_CRATES: &[&str] = &["mnl", "nftnl"];

const COLORS: ColoredLevelConfig = ColoredLevelConfig {
    error: Color::Red,
    warn: Color::Yellow,
    info: Color::Green,
    debug: Color::Blue,
    trace: Color::Black,
};

#[cfg(not(windows))]
const LINE_SEPARATOR: &str = "\n";

#[cfg(windows)]
const LINE_SEPARATOR: &str = "\r\n";

const DATE_TIME_FORMAT_STR: &str = "[%Y-%m-%d %H:%M:%S%.3f]";

pub fn init_logger(
    log_level: log::LevelFilter,
    log_file: Option<&PathBuf>,
    output_timestamp: bool,
) -> Result<(), Error> {
    let mut top_dispatcher = fern::Dispatch::new().level(log_level);
    for silenced_crate in SILENCED_CRATES {
        top_dispatcher = top_dispatcher.level_for(*silenced_crate, log::LevelFilter::Warn);
    }
    for silenced_crate in SLIGHTLY_SILENCED_CRATES {
        top_dispatcher = top_dispatcher.level_for(*silenced_crate, one_level_quieter(log_level));
    }

    let stdout_formatter = Formatter {
        output_timestamp,
        output_color: true,
    };
    let stdout_dispatcher = fern::Dispatch::new()
        .format(move |out, message, record| stdout_formatter.output_msg(out, message, record))
        .chain(io::stdout());
    top_dispatcher = top_dispatcher.chain(stdout_dispatcher);

    if let Some(ref log_file) = log_file {
        rotate_log(log_file).map_err(Error::RotateLog)?;
        let file_formatter = Formatter {
            output_timestamp: true,
            output_color: false,
        };
        let f = fern::log_file(log_file).map_err(|source| Error::WriteFile {
            path: log_file.display().to_string(),
            source,
        })?;
        let file_dispatcher = fern::Dispatch::new()
            .format(move |out, message, record| file_formatter.output_msg(out, message, record))
            .chain(Output::file(f, LINE_SEPARATOR));
        top_dispatcher = top_dispatcher.chain(file_dispatcher);
    }
    top_dispatcher.apply().map_err(Error::SetLoggerError)?;
    Ok(())
}

fn one_level_quieter(level: log::LevelFilter) -> log::LevelFilter {
    use log::LevelFilter::*;
    match level {
        Off => Off,
        Error => Off,
        Warn => Error,
        Info => Warn,
        Debug => Info,
        Trace => Debug,
    }
}

#[derive(Default, Debug)]
struct Formatter {
    pub output_timestamp: bool,
    pub output_color: bool,
}

impl Formatter {
    fn get_timetsamp_fmt(&self) -> &str {
        if self.output_timestamp {
            DATE_TIME_FORMAT_STR
        } else {
            &""
        }
    }

    fn get_record_level(&self, level: log::Level) -> Box<dyn fmt::Display> {
        if self.output_color && cfg!(not(windows)) {
            Box::new(COLORS.color(level))
        } else {
            Box::new(level)
        }
    }

    pub fn output_msg(
        &self,
        out: fern::FormatCallback<'_>,
        message: &fmt::Arguments<'_>,
        record: &log::Record<'_>,
    ) {
        let message = escape_newlines(format!("{}", message));

        out.finish(format_args!(
            "{}[{}][{}] {}",
            chrono::Local::now().format(self.get_timetsamp_fmt()),
            record.target(),
            self.get_record_level(record.level()),
            message,
        ))
    }
}

#[cfg(not(windows))]
fn escape_newlines(text: String) -> String {
    text
}

#[cfg(windows)]
fn escape_newlines(text: String) -> String {
    text.replace("\n", LINE_SEPARATOR)
}
