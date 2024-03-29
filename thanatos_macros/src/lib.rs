use proc_macro::TokenStream;

#[proc_macro_derive(Archetype)]
pub fn derive_answer_fn(input: TokenStream) -> TokenStream {
    format!("impl_archetype!({});", input.to_string())
        .parse()
        .unwrap()
}
