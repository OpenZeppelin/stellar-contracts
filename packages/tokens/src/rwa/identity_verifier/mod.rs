// TODO: mention that for our suggested implementation, we also require
// `IdentityStorage`, and we provide the setter/getter for that contract in our
// storage.rs file and also present in our examples folder for RWA. However,
// based on the implementation, `IdentityStorage` may not be required, hence it
// is not a part of the trait.
pub trait IdentityVerifier {
    /// Sets the claim topics and issuers contract of the token.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `claim_topics_and_issuers` - The address of the claim topics and
    ///   issuers contract to set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["claim_topics_issuers_set", claim_topics_and_issuers:
    ///   Address]`
    /// * data - `[]`
    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address);

    /// Returns the Claim Topics and Issuers contract linked to the token.
    ///
    /// # Errors
    ///
    /// * [`RWAError::ClaimTopicsAndIssuersNotSet`] - When the claim topics and
    ///   issuers contract is not set.
    fn claim_topics_and_issuers(e: &Env) -> Address;
}
