extern crate proc_macro;

use crate::proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Punct, Spacing};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Attribute, Data, Fields, Lit, Meta};

#[proc_macro_derive(Digest, attributes(digest_prefix))]
pub fn digest_macro_fn(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_digest_macro(&ast)
}

fn impl_digest_macro(ast: &syn::DeriveInput) -> TokenStream {
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

                    let bytes = fields.named.iter().map(|field| attr_to_bytes(&field.attrs));
                    (idents, types, bytes)
                }
                _ => panic!("Only supporting named fields."),
            };

            let gen = quote! {
                    impl ::digest::Digest for #name
                        where #(#types: ::digest::IntoDigestInput),*
                    {
                        fn digest(self) -> ::multihash::Multihash {
                            let mut digests = vec![];
                            #(digests.push(::digest::field_digest(self.#idents, #bytes.to_vec())););*

                            digests.sort();

                            let res = digests.into_iter().fold(vec![], |mut res, digest| {
                                res.append(&mut digest.into_bytes());
                                res
                            });

                            ::digest::digest(&res)
                        }
                    }
            };
            gen.into()
        }
        Data::Enum(data) => {
            let unit_variant_idents = data
                .variants
                .iter()
                .filter(|variant| variant.fields.is_empty())
                .map(|variant| &variant.ident);

            let unit_variant_bytes = data
                .variants
                .iter()
                .filter(|variant| variant.fields.is_empty())
                .map(|variant| attr_to_bytes(&variant.attrs));

            let tuple_variant_idents = data
                .variants
                .iter()
                .filter(|variant| match variant.fields {
                    Fields::Unnamed(_) => true,
                    _ => false,
                })
                .map(|variant| &variant.ident);

            let tuple_variant_bytes = data
                .variants
                .iter()
                .filter(|variant| match variant.fields {
                    Fields::Unnamed(_) => true,
                    _ => false,
                })
                .map(|variant| attr_to_bytes(&variant.attrs));

            let gen = quote! {
                    impl ::digest::Digest for #name
                    {
                        fn digest(self) -> ::multihash::Multihash {
                            let bytes = match self {
                                #(Self::#unit_variant_idents => #unit_variant_bytes.to_vec()),*,
                                #(Self::#tuple_variant_idents(data) => {
                                        let mut bytes = #tuple_variant_bytes.to_vec();
                                        bytes.append(&mut data.digest().into_bytes());
                                        bytes
                                }),*
                            };

                            ::digest::digest(&bytes)
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

fn attr_to_bytes(attrs: &[Attribute]) -> Bytes {
    let attr = attrs
        .get(0)
        .expect("digest_prefix attribute must be the only attribute present on all fields");
    let meta = attr.parse_meta().expect("Attribute is malformed");

    if let Meta::NameValue(name_value) = meta {
        if name_value.path.is_ident("digest_prefix") {
            if let Lit::Str(lit_str) = name_value.lit {
                let str = lit_str.value();
                let bytes =
                    ::hex::decode(&str).expect("digest_prefix value should be in hex format");
                return Bytes(bytes);
            }
        }
    }
    panic!("Only `digest_prefix = \"0102..0A\"` attributes are supported");
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
