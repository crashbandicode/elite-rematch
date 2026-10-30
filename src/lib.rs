//! elite-rematch: keeps the online rematch in Smash Ultimate 13.0.5 when a
//! player enters or leaves Elite Smash. The post-match popup that announces
//! the change is shown with the ordinary rematch buttons; the Elite Smash
//! text stays as the indicator. UI only: no rating, network or match state
//! changes, and no random draws.
pub mod model;
#[cfg(target_os = "switch")]
mod runtime;
pub mod share;
