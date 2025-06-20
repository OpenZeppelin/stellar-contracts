// Base trait that defines the minimal interface needed by RWA tokens
// it may also make sense to merge this with `IERC3643`. To be compatible with base RWA as much as possible,
// we separated this into its own trait as in `IERC3643`.
pub trait IdentityVerifier {
    // Checks if a user address is verified according to the current requirements
    // This is the only function required by RWA tokens for transfer validation
    fn is_verified(user_address: Address) -> bool;
}
