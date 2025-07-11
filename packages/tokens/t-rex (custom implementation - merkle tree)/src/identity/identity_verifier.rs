/// Base trait that defines the minimal interface needed by RWA tokens
/// This is the only function required by T-Rex tokens for transfer validation
pub trait IdentityVerifier {
    /// Checks if a user address is verified according to the current requirements
    fn is_verified(e: &Env, account: Address) -> bool;
}
