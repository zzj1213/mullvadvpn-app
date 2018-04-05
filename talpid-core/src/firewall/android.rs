use super::{Firewall, SecurityPolicy};

error_chain!{}

/// The Android implementation for the `Firewall` trait.
pub struct AndroidFirewall;

impl Firewall for AndroidFirewall {
    type Error = Error;

    fn new() -> Result<Self> {
        Ok(AndroidFirewall)
    }

    fn apply_policy(&mut self, _policy: SecurityPolicy) -> Result<()> {
        Ok(())
    }

    fn reset_policy(&mut self) -> Result<()> {
        Ok(())
    }
}
