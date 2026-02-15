use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput};

#[proc_macro_derive(CollectionClient, attributes(element))]
pub fn collection_client_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    match ast.data {
        Data::Struct(_) => {
            let name = &ast.ident;
            let expanded = quote! {
                impl crate::CollectionClient for #name {}
                impl crate::Data for #name {

                    fn data(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, Self::Entity>> {
                        self.data.read().unwrap()
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
