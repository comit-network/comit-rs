extern crate proc_macro;

use crate::proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Punct, Spacing};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Attribute, Data, Fields, Lit, Meta, MetaList, NestedMeta, Type};

#[proc_macro_derive(Digest, attributes(digest))]
pub fn digest_macro_fn(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_digest_macro(&ast)
}

fn impl_digest_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let hash_type = extract_hash_type(&ast.attrs);

    match &ast.data {
        Data::Struct(data) => {
            let (idents, types, bytes) = match &data.fields {
                Fields::Named(fields) => {
                    let fields = fields.named.iter().filter(|field| {
                        let meta_list = extract_meta_list(&field.attrs);

                        let path = &meta_list.nested.first();
                        if let Some(NestedMeta::Meta(Meta::Path(path))) = path {
                            if path.is_ident("ignore") {
                                return false;
                            }
                        }
                        true
                    });

                    let idents = fields
                        .clone()
                        .map(|field| field.ident.as_ref().expect("Named field"));

                    let types = fields.clone().map(|field| &field.ty);

                    let bytes = fields.map(|field| extract_bytes(&field.attrs));
                    (idents, types, bytes)
                }
                _ => panic!("Only supporting named fields."),
            };

            let gen = quote! {
                    impl ::digest::Digest for #name
                        where #(#types: ::digest::ToDigestInput),*
                    {
                        type Hash = #hash_type;

                        fn digest(&self) -> Self::Hash {
                            use ::digest::{Hash, ToDigestInput};
                            let mut digests = vec![];
                            #(digests.push(::digest::field_digest::<_, Self::Hash>(&self.#idents, #bytes.to_vec())););*

                            digests.sort();

                            let bytes = digests.into_iter().fold(vec![], |mut bytes, digest| {
                                bytes.append(&mut digest.to_digest_input());
                                bytes
                            });

                            Self::Hash::hash(&bytes)
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
                .map(|variant| extract_bytes(&variant.attrs));

            let tuple_variant_idents = data
                .variants
                .iter()
                .filter(|variant| matches!(variant.fields, Fields::Unnamed(_)))
                .map(|variant| &variant.ident);

            let tuple_variant_bytes = data
                .variants
                .iter()
                .filter(|variant| matches!(variant.fields, Fields::Unnamed(_)))
                .map(|variant| extract_bytes(&variant.attrs));

            let gen = quote! {
                    impl ::digest::Digest for #name
                    {
                        type Hash = #hash_type;

                        fn digest(&self) -> Self::Hash {
                            use ::digest::{Hash, ToDigestInput};

                            let bytes = match self.clone() {
                                #(Self::#unit_variant_idents => #unit_variant_bytes.to_vec()),*,
                                #(Self::#tuple_variant_idents(data) => {
                                        let mut bytes = #tuple_variant_bytes.to_vec();
                                        bytes.append(&mut data.digest().to_digest_input());
                                        bytes
                                }),*
                            };

                            Self::Hash::hash(&bytes)
                        }
                    }
            };
            gen.into()
        }
        _ => {
            panic!("Digest derive macro only supports structs & enums.");
        }
    }
}

fn extract_hash_type(attrs: &[Attribute]) -> Type {
    let meta_list = extract_meta_list(attrs);

    let path = &meta_list.nested.first();
    if let Some(NestedMeta::Meta(Meta::NameValue(name_value))) = path {
        if name_value.path.is_ident("hash") {
            if let Lit::Str(ref lit_str) = name_value.lit {
                if let Ok(hash_type) = syn::parse_str::<Type>(&lit_str.value()) {
                    return hash_type;
                }
            }
            panic!("hash type could not be resolved. Expected format: `#[digest(hash = \"MyHashType\")]` ")
        } else {
            panic!("Only `hash` identifier is supported for `digest()` outer attribute. Expected format: `#[digest(hash = \"MyHashType\")]`")
        }
    } else {
        panic!("Could not find element inside `digest` attribute. Expected format: `#[digest(hash = \"MyHashType\")]`")
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

fn extract_bytes(attrs: &[Attribute]) -> Bytes {
    let meta_list = extract_meta_list(attrs);

    let path = &meta_list.nested.first();
    if let Some(NestedMeta::Meta(Meta::NameValue(name_value))) = path {
        if name_value.path.is_ident("prefix") {
            if let Lit::Str(ref lit_str) = name_value.lit {
                let str = lit_str.value();
                let bytes = ::hex::decode(&str).expect("prefix value should be in hex format");
                return Bytes(bytes);
            }
            panic!(
                "prefix could not be resolved. Expected format: `#[digest(prefix = \"0102..0A\")]`"
            )
        } else {
            panic!("Only `prefix` identifier is supported for `digest()` field attribute. Expected format: `#[digest(prefix = \"0102..0A\")]`")
        }
    } else {
        panic!("Could not find element inside `digest` attribute. Expected format: `#[digest(hash = \"MyHashType\")]`")
    }
}

fn extract_meta_list(attrs: &[Attribute]) -> MetaList {
    attrs
        .iter()
        .find_map(|attr| {
            attr.parse_meta()
                .ok()
                .map(|meta| {
                    if let Meta::List(meta_list) = meta {
                        if meta_list.path.is_ident("digest") {
                            return Some(meta_list);
                        }
                    };
                    None
                })
                .unwrap_or(None)
        })
        .expect("Could not find `digest` attribute.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implement_to_token() {
        let bytes = Bytes(vec![0u8, 1u8, 2u8, 3u8]);

        let tokens = quote!(#bytes);
        assert_eq!(tokens.to_string(), "[0u8 , 1u8 , 2u8 , 3u8]");
    }
}
