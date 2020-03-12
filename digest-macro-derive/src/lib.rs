extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;
use syn::{Data, Fields, Lit, Meta};

#[proc_macro_derive(RootDigestMacro, attributes(digest_bytes))]
pub fn root_digest_macro_fn(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_root_digest_macro(&ast)
}

fn impl_root_digest_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    if let Data::Struct(data) = &ast.data {
        if let Fields::Named(fields) = &data.fields {
            let idents = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("Named field"));

            let types = fields.named.iter().map(|field| &field.ty);

            let bytes_str = fields.named.iter().map(|field| {
                let attr = field
                    .attrs
                    .get(0)
                    .expect("digest_bytes attribute must be present on all fields");
                let meta = attr.parse_meta().expect("Attribute is malformed");

                if let Meta::NameValue(name_value) = meta {
                    if name_value.path.is_ident("digest_bytes") {
                        if let Lit::Str(lit_str) = name_value.lit {
                            let str = lit_str.value();
                            // Ensure it is a correct format
                            let _ = ::hex::decode(&str)
                                .expect("digest_bytes value should be in hex format");
                            return str;
                        }
                    }
                }
                panic!("Only `digest_bytes = \"0102..0A\"` attributes are supported");
            });

            let gen = quote! {
            impl ::digest::RootDigest for #name
                 where #(#types: ::digest::FieldDigest),*
                 {
                    fn root_digest(self) -> Multihash {
                        use ::digest::hex;
                        let mut digests = vec![];
                        #(digests.push(self.#idents.field_digest(hex::decode(#bytes_str).unwrap())););*

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
