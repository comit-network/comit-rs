extern crate proc_macro;

use crate::proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Punct, Spacing};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Attribute, Data, Fields, Lit, Meta};

#[proc_macro_derive(RootDigestMacro, attributes(digest_bytes))]
pub fn root_digest_macro_fn(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_root_digest_macro(&ast)
}

fn impl_root_digest_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    match &ast.data {
        Data::Struct(data) => {
            let (idents, types, bytes) = match &data.fields {
                Fields::Named(fields) => {
                    let idents = fields
                        .named
                        .iter()
                        .map(|field| field.ident.as_ref().expect("Named field"));

                    let types = fields.named.iter().map(|field| &field.ty);

                    let bytes = fields.named.iter().map(|field| {
                        let attr = field
                            .attrs
                            .get(0)
                            .expect("digest_bytes attribute must be present on all fields");

                        attr_to_bytes(attr)
                    });
                    (idents, types, bytes)
                }
                _ => panic!("Only supporting named fields."),
            };

            let gen = quote! {
                    impl ::digest::RootDigest for #name
                        where #(#types: ::digest::FieldDigest),*
                    {
                        fn root_digest(self) -> Multihash {
                            let mut digests = vec![];
                            #(digests.push(self.#idents.field_digest(#bytes.to_vec())););*

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
        }
        Data::Enum(data) => {
            let idents = data.variants.iter().map(|variant| &variant.ident);

            let bytes = data.variants.iter().map(|variant| {
                let attr = variant
                    .attrs
                    .get(0)
                    .expect("digest_bytes attribute must be present on all fields");
                attr_to_bytes(attr)
            });

            let gen = quote! {
                    impl ::digest::RootDigest for #name
                    {
                        fn root_digest(self) -> Multihash {
                            let bytes = match self {
                                #(Self::#idents => #bytes.to_vec()),*
                            };

                            digest(&bytes)
                        }
                    }
            };
            gen.into()
        }
        _ => {
            panic!("DigestRootMacro only supports structs & enums.");
        }
    }
}

struct Bytes(Vec<u8>);

impl ToTokens for Bytes {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut inner_tokens = proc_macro2::TokenStream::new();
        inner_tokens.append_separated(&self.0, Punct::new(',', Spacing::Alone));
        let group = Group::new(Delimiter::Bracket, inner_tokens);
        tokens.append(group);
    }
}

fn attr_to_bytes(attr: &Attribute) -> Bytes {
    let meta = attr.parse_meta().expect("Attribute is malformed");

    if let Meta::NameValue(name_value) = meta {
        if name_value.path.is_ident("digest_bytes") {
            if let Lit::Str(lit_str) = name_value.lit {
                let str = lit_str.value();
                let bytes =
                    ::hex::decode(&str).expect("digest_bytes value should be in hex format");
                return Bytes(bytes);
            }
        }
    }
    panic!("Only `digest_bytes = \"0102..0A\"` attributes are supported");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implement_to_token() {
        let bytes = Bytes(vec![0u8, 1u8, 2u8, 3u8]);

        let tokens = quote!(#bytes);
        assert_eq!(tokens.to_string(), "[ 0u8 , 1u8 , 2u8 , 3u8 ]");
    }
}
