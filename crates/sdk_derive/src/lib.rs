use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Type};

fn get_element_type(attrs: &[syn::Attribute]) -> Type {
    for attr in attrs {
        if attr.path().is_ident("element") {
            return attr
                .parse_args::<Type>()
                .expect("expected type argument, e.g. #[element(Item)]");
        }
    }
    panic!("missing #[element(Type)] attribute");
}

fn get_key_type(attrs: &[syn::Attribute]) -> Type {
    for attr in attrs {
        if attr.path().is_ident("key") {
            return attr
                .parse_args::<Type>()
                .expect("expected type argument, e.g. #[key(String)]");
        }
    }
    syn::parse_quote!(String)
}

fn get_data_path(attrs: &[syn::Attribute]) -> Expr {
    for attr in attrs {
        if attr.path().is_ident("data_path") {
            return attr
                .parse_args::<Expr>()
                .expect("expected expression path, e.g. #[data_path(self.0.data)]");
        }
    }
    syn::parse_quote!(self.0.data)
}

#[proc_macro_derive(CollectionClient, attributes(element, key, data_path))]
pub fn collection_client_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    match ast.data {
        Data::Struct(_) => {
            let name = &ast.ident;
            let entity_type = get_element_type(&ast.attrs);
            let key_type = get_key_type(&ast.attrs);
            let data_path = get_data_path(&ast.attrs);
            let expanded = quote! {
                impl crate::client::private::Sealed for #name {}
                impl crate::CollectionClient for #name {}
                impl crate::Data for #name {
                    type Entity = #entity_type;
                    type Key = #key_type;

                    fn data(&self) -> std::sync::Arc<std::collections::HashMap<Self::Key, Self::Entity>> {
                        #data_path.load_full()
                    }
                }
            };
            expanded.into()
        }
        _ => panic!("CollectionClient derive can only be used on struct"),
    }
}

// #[proc_macro_derive(FromRequestError)]
// pub fn from_request_error_derive(input: TokenStream) -> TokenStream {
//     let ast = syn::parse_macro_input!(input as DeriveInput);
//
//     match ast.data {
//         Data::Enum(ref _data) => {
//             let name = &ast.ident;
//             let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
//             let expanded = quote! {
//                 impl #impl_generics From<RequestError> for #name #ty_generics #where_clause {
//                     fn from(value: RequestError) -> Self {
//                         if let RequestError::ResponseError(ref schema) = value {
//                             return Self::try_from(schema.error.code as isize)
//                                 .unwrap_or(Self::UnhandledError(value));
//                             }
//                         Self::UnhandledError(value)
//                     }
//                 }
//             };
//             expanded.into()
//         }
//         _ => panic!("FromRequestError derive can only be used on enums"),
//     }
// }
