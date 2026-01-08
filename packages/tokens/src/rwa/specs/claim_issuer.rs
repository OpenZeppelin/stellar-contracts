// netanel's ideas
// Invariants:
//If ClaimIssuerStorageKey::Pairs(SigningKey) exist, then it points to a non-empty vector.
//A key which is part of a vector of ClaimIssuerStorageKey:: Topics(u32) must have at least one associated registry for that topic
// (i.e., if the topic is 25 the value of  ClaimIssuerStorageKey::Pairs(SigningKey) is Vec<(Topic, Registry) and we want Vec<(25, Registry) to be non-empty).
// roperties:
// The data structures SigningKey (tracks the topic-registry pairs for which a given signing key is authorized) and ClaimIssuerStorageKey (tracks which signing keys
// are authorized to sign claims for a specific topic) are correctly correlated;
