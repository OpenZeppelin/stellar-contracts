/// Contract for verifying WebAuthn Authentication Assertions.
///
/// This contract verifies signatures generated during WebAuthn authentication
/// ceremonies as specified in the https://www.w3.org/TR/webauthn-2/.
///
/// For blockchain use cases, the following WebAuthn validations are
/// intentionally omitted:
///
/// * Origin validation: Origin verification in `clientDataJSON` is omitted as
///   blockchain contexts rely on authenticator and dapp frontend enforcement.
///   Standard authenticators implement proper origin validation.
/// * RP ID hash validation: Verification of `rpIdHash` in authenticatorData
///   against expected RP ID hash is omitted. This is typically handled by
///   platform-level security measures. Including an expiry timestamp in signed
///   data is recommended for enhanced security.
/// * Signature counter: Verification of signature counter increments is
///   omitted. While useful for detecting credential cloning, on-chain
///   operations typically include nonce protection, making this check
///   redundant.
/// * Extension outputs: Extension output value verification is omitted as these
///   are not essential for core authentication security in blockchain
///   applications.
/// * Attestation: Attestation object verification is omitted as this
///   implementation focuses on authentication (`webauthn.get`) rather than
///   registration ceremonies.
///
///   Adapted from:
///   * https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/utils/cryptography/WebAuthn.sol
///   * https://github.com/kalepail/passkey-kit/blob/next/contracts/smart-wallet/src/verify.rs
use soroban_sdk::{contracterror, contracttype, panic_with_error, Bytes, BytesN, Env, String};

use crate::verifiers::utils::{base64_url_encode, extract_from_bytes};

/// Bit 0 of the authenticator data flags: "User Present" bit.
pub const AUTH_DATA_FLAGS_UP: u8 = 0x01;
/// Bit 2 of the authenticator data flags: "User Verified" bit.
pub const AUTH_DATA_FLAGS_UV: u8 = 0x04;
/// Bit 3 of the authenticator data flags: "Backup Eligibility" bit.
pub const AUTH_DATA_FLAGS_BE: u8 = 0x08;
/// Bit 4 of the authenticator data flags: "Backup State" bit.
pub const AUTH_DATA_FLAGS_BS: u8 = 0x10;

/// Max. length of client_data
pub const CLIENT_DATA_MAX_LEN: usize = 1024;
/// Min. length of authenticator_data
pub const AUTHENTICATOR_DATA_MIN_LEN: usize = 37;

/// Error types for WebAuthn verification operations.
#[contracterror]
#[repr(u32)]
pub enum WebAuthnError {
    /// The signature payload is invalid or has incorrect format.
    SignaturePayloadInvalid = 3110,
    /// The client data exceeds the maximum allowed length.
    ClientDataTooLong = 3111,
    /// Failed to parse JSON from client data.
    JsonParseError = 3112,
    /// The type field in client data is not "webauthn.get".
    TypeFieldInvalid = 3113,
    /// The challenge in client data does not match expected value.
    ChallengeInvalid = 3114,
    /// The authenticator data format is invalid or too short.
    AuthDataFormatInvalid = 3115,
    /// The User Present (UP) bit is not set in authenticator flags.
    PresentBitNotSet = 3116,
    /// The User Verified (UV) bit is not set in authenticator flags.
    VerifiedBitNotSet = 3117,
    /// Invalid relationship between Backup Eligibility and State bits.
    BackupEligibilityAndStateNotSet = 3118,
}

/// Parsed client data JSON structure for WebAuthn authentication.
///
/// Contains the essential fields from the client data JSON that need to be
/// validated during WebAuthn assertion verification.
#[derive(serde::Deserialize)]
pub struct ClientDataJson<'a> {
    /// Base64url-encoded challenge value that must match the signature payload.
    pub challenge: &'a str,
    /// Type of WebAuthn operation, must be "webauthn.get" for authentication.
    #[serde(rename = "type")]
    pub type_field: &'a str,
}

/// WebAuthn signature data structure containing all components needed for
/// verification.
///
/// This structure encapsulates the signature and associated data generated
/// during a WebAuthn authentication ceremony.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct WebAuthnSigData {
    /// The cryptographic signature (64 bytes for secp256r1).
    pub signature: BytesN<64>,
    /// Raw authenticator data from the WebAuthn response.
    pub authenticator_data: Bytes,
    /// Raw client data JSON from the WebAuthn response.
    pub client_data: Bytes,
}

/// Validates that the type field in client data matches "webauthn.get".
///
/// This implements Step 11 of the WebAuthn assertion verification procedure.
/// The type field must be exactly "webauthn.get" for authentication ceremonies.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `client_data_json` - Parsed client data JSON structure.
///
/// # Errors
///
/// * [`WebAuthnError::TypeFieldInvalid`] - When the type field is not
///   "webauthn.get".
///
/// # Reference
///
/// Step 11 in <https://www.w3.org/TR/webauthn-2/#sctn-verifying-assertion>
pub fn validate_expected_type(e: &Env, client_data_json: &ClientDataJson) {
    let type_field = String::from_str(e, "webauthn.get");
    if String::from_str(e, client_data_json.type_field) != type_field {
        panic_with_error!(e, WebAuthnError::TypeFieldInvalid)
    }
}

/// Validates that the challenge in client data matches the expected signature
/// payload.
///
/// This implements Step 12 of the WebAuthn assertion verification procedure.
/// The challenge must be the base64url-encoded representation of the first 32
/// bytes of the signature payload.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `client_data_json` - Parsed client data JSON structure.
/// * `signature_payload` - The expected signature payload bytes.
///
/// # Errors
///
/// * [`WebAuthnError::SignaturePayloadInvalid`] - When the signature payload is
///   too short or malformed.
/// * [`WebAuthnError::ChallengeInvalid`] - When the challenge doesn't match the
///   expected base64url-encoded signature payload.
///
/// # Reference
///
/// Step 12 in <https://www.w3.org/TR/webauthn-2/#sctn-verifying-assertion>
pub fn validate_challenge(e: &Env, client_data_json: &ClientDataJson, signature_payload: &Bytes) {
    let signature_payload: BytesN<32> = extract_from_bytes(e, signature_payload, 0..32)
        .unwrap_or_else(|| panic_with_error!(e, WebAuthnError::SignaturePayloadInvalid));

    // base64 url encoded value of `signature_payload: Hash<32>`
    let mut expected_challenge = [0u8; 43];

    base64_url_encode(&mut expected_challenge, &signature_payload.to_array());

    if client_data_json.challenge.as_bytes() != expected_challenge {
        panic_with_error!(e, WebAuthnError::ChallengeInvalid)
    }
}

/// Validates that the User Present (UP) bit is set in authenticator flags.
///
/// This implements Step 16 of the WebAuthn assertion verification procedure.
/// The UP bit indicates that the user was present during the authentication
/// ceremony (e.g., touched the authenticator).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `flags` - The flags byte from authenticator data.
///
/// # Errors
///
/// * [`WebAuthnError::PresentBitNotSet`] - When the User Present bit is not
///   set.
///
/// # Reference
///
/// Step 16 in <https://www.w3.org/TR/webauthn-2/#sctn-verifying-assertion>
pub fn validate_user_present_bit_set(e: &Env, flags: u8) {
    // Validates that the <https://www.w3.org/TR/webauthn-2/#up> bit is set.
    if (flags & AUTH_DATA_FLAGS_UP) == 0 {
        panic_with_error!(e, WebAuthnError::PresentBitNotSet)
    }
}

/// Validates that the User Verified (UV) bit is set in authenticator flags.
///
/// This implements Step 17 of the WebAuthn assertion verification procedure.
/// The UV bit indicates whether the user was verified using a stronger
/// identification method (biometrics, PIN, password).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `flags` - The flags byte from authenticator data.
///
/// # Errors
///
/// * [`WebAuthnError::VerifiedBitNotSet`] - When the User Verified bit is not
///   set.
///
/// # Security Note
///
/// The choice of requiring UV represents a security vs. usability tradeoff.
/// For blockchain applications handling valuable assets, requiring UV is
/// generally safer. However, for routine operations or when using hardware
/// authenticators without verification capabilities, `UV=0` may be acceptable.
///
/// # Reference
///
/// Step 17 in <https://www.w3.org/TR/webauthn-2/#sctn-verifying-assertion>
pub fn validate_user_verified_bit_set(e: &Env, flags: u8) {
    if (flags & AUTH_DATA_FLAGS_UV) == 0 {
        panic_with_error!(e, WebAuthnError::VerifiedBitNotSet)
    }
}

/// Validates the relationship between Backup Eligibility (BE) and Backup
/// State (BS) bits according to the WebAuthn specification.
///
/// The check enforces that if a credential is backed up (BS=1), it must also
/// be eligible for backup (BE=1). This prevents unauthorized credential backup
/// and ensures compliance with the WebAuthn spec.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `flags` - The flags byte from authenticator data.
///
/// # Errors
///
/// * [`WebAuthnError::BackupEligibilityAndStateNotSet`] - When BE=0 and BS=1,
///   which is an invalid state.
///
/// # Valid States
///
/// * `BE=1`, `BS=0`: Credential is eligible but not backed up
/// * `BE=1`, `BS=1`: Credential is eligible and backed up
/// * `BE=0`, `BS=0`: Credential is not eligible and not backed up
///
/// # Invalid State
///
/// * `BE=0`, `BS=1`: Credential is backed up but not eligible for backup
///
/// # Security Note
///
/// While the WebAuthn spec defines this relationship between BE and BS bits,
/// validating it is not explicitly required as part of the core verification
/// procedure. Some implementations may choose to skip this check for broader
/// authenticator compatibility or when the application's threat model doesn't
/// consider credential syncing a major risk.
pub fn validate_backup_eligibility_and_state(e: &Env, flags: u8) {
    if (flags & AUTH_DATA_FLAGS_BE) == 0 && (flags & AUTH_DATA_FLAGS_BS) != 0 {
        panic_with_error!(e, WebAuthnError::BackupEligibilityAndStateNotSet)
    }
}

/// Performs complete verification of a WebAuthn Authentication Assertion.
///
/// This function implements the WebAuthn assertion verification procedure as
/// specified in the W3C WebAuthn specification, with blockchain-specific
/// adaptations.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signature_payload` - The data that was signed (first 32 bytes used as
///   challenge).
/// * `pub_key` - The public key (65 bytes for secp256r1).
/// * `sig_data` - WebAuthnSigData containing signature and associated data.
///
/// # Returns
///
/// Returns `true` if verification succeeds.
///
/// # Errors
///
/// * [`WebAuthnError::ClientDataTooLong`] - When client data exceeds maximum
///   length.
/// * [`WebAuthnError::JsonParseError`] - When client data JSON is malformed.
/// * [`WebAuthnError::AuthDataFormatInvalid`] - When authenticator data is too
///   short.
/// * Other validation errors from individual validation functions.
///
/// # Verification Steps
///
/// 1. Type is "webauthn.get"
/// 2. Challenge matches the expected value
/// 3. Cryptographic signature is valid for the given public key
/// 4. Confirming physical user presence during authentication
/// 5. Confirming stronger user authentication (biometrics/PIN)
/// 6. Backup Eligibility (BE) and Backup State (BS) bits relationship is valid
///
/// # Reference
///
/// <https://www.w3.org/TR/webauthn-2/#sctn-verifying-assertion>
pub fn verify(
    e: &Env,
    signature_payload: &Bytes,
    pub_key: &BytesN<65>,
    sig_data: &WebAuthnSigData,
) -> bool {
    let WebAuthnSigData { signature, authenticator_data, client_data } = sig_data;

    // Assume that client_data < 1KB, because:
    // - challenge: 43 bytes (equals to the base64 url encoded value of
    //   `signature_payload: Hash<32>`)
    // - type: 12 bytes ("webauthn.get")
    // - crossOrigin: optional boolean
    // - tokenBinding: optional field with a variable length
    // - origin: variable length, but in almost all cases will fit into a couple of
    //   dozens bytes
    //
    // https://www.w3.org/TR/webauthn-2/#client-data
    if client_data.len() > CLIENT_DATA_MAX_LEN as u32 {
        panic_with_error!(e, WebAuthnError::ClientDataTooLong)
    }

    let client_data_json = client_data.to_buffer::<CLIENT_DATA_MAX_LEN>();
    let (client_data_json, _): (ClientDataJson, _) =
        serde_json_core::de::from_slice(client_data_json.as_slice())
            .unwrap_or_else(|_| panic_with_error!(e, WebAuthnError::JsonParseError));

    validate_expected_type(e, &client_data_json);
    validate_challenge(e, &client_data_json, signature_payload);

    // Verify authenticator data has sufficient length (37 bytes minimum):
    // - 32 bytes for rpIdHash
    // - 1 byte for flags
    // - 4 bytes for signature counter
    //
    // https://www.w3.org/TR/webauthn-2/#authenticator-data
    if authenticator_data.len() < AUTHENTICATOR_DATA_MIN_LEN as u32 {
        panic_with_error!(e, WebAuthnError::AuthDataFormatInvalid)
    }

    // Safe because of the check above.
    let flags = authenticator_data.get(32).expect("32 byte to be present");

    validate_user_present_bit_set(e, flags);
    validate_user_verified_bit_set(e, flags);
    validate_backup_eligibility_and_state(e, flags);

    // Step 19 in https://www.w3.org/TR/webauthn-2/#sctn-verifying-assertion.
    let client_data_hash = e.crypto().sha256(client_data);

    // Step 20 in https://www.w3.org/TR/webauthn-2/#sctn-verifying-assertion.
    let mut message_digest = authenticator_data.clone();
    message_digest.extend_from_array(&client_data_hash.to_array());

    e.crypto().secp256r1_verify(pub_key, &e.crypto().sha256(&message_digest), signature);

    true
}
