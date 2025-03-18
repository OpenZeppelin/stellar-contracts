mod derive;

use derive::derive_upgradeable;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Upgradeable, attributes(migrateable))]
pub fn upgradeable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_upgradeable(&input).into()
}
