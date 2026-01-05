// todo
// maybe make a non-panicking version of verify_identity

// if there is some invalidity it should panic.

// more specifically, it seems to be disjunctive over claims

// but then is it conjungtive over the different issuers?

pub fn verify_identity_panics_if_invalid_claim(e: Env) {
    let account = nondet_address();
    let 
    storage::verify_identity(&e, &account);
    cvlr_assert!(false);
}

