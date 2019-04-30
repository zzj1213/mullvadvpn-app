//#[cfg(feature = "serde")]
//#[cfg_attr(feature = "serde", macro_use)]
extern crate serde;

#[macro_use]
extern crate serde_derive;

extern crate derive_try_from_primitive;

pub mod tinc_tcp_stream;
pub mod control;
pub mod listener;
pub use self::listener::EventType;
pub use self::listener::spawn;