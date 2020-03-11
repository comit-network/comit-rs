extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;
use syn::{Data, Fields};

#[proc_macro_derive(DigestRootMacro)]
pub fn digest_root_macro_fn(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_digest_root_macro(&ast)
}

fn impl_digest_root_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    if let Data::Struct(data) = &ast.data {
        if let Fields::Named(fields) = &data.fields {
            let idents = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("Named field"));

            let types = fields.named.iter().map(|field| &field.ty);

            let gen = quote! {
            impl ::digest::DigestRoot for #name
                 where #(#types: ::digest::DigestField),*
                 {
                    fn digest_root(self) -> Multihash {
                        let mut digests = vec![];
                        #(digests.push(self.#idents.digest_field(stringify!(#idents).into())););*

                        digests.sort();

                        let res = digests.into_iter().fold(vec![], |mut res, digest| {
                            res.append(&mut digest.into_bytes());
                            res
                        });

                        digest(&res)
                    }
                }
            };
            gen.into()
        } else {
            panic!("DigestRootMacro only supports named filed, ie, no new types, tuples structs/variants or unit struct/variants.");
        }
    } else {
        panic!("DigestRootMacro only supports structs.");
    }
}
