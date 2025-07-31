//!Automatic trait extension macro for wrapper types
#![warn(missing_docs)]
#![allow(clippy::style)]

use proc_macro::TokenStream;

use quote::quote;

struct TypeInfo {
    ident: syn::Ident,
    generics: Option<syn::AngleBracketedGenericArguments>,
    reference: Option<syn::Lifetime>,
    mutability: Option<syn::token::Mut>
}

fn generate_self_trait_bound(generic_name: syn::Ident, trait_name: &syn::Ident) -> syn::GenericArgument {
    let mut segments = syn::punctuated::Punctuated::new();
    segments.push(syn::PathSegment {
        ident: trait_name.clone(),
        arguments: syn::PathArguments::None,
    });

    let mut bounds = syn::punctuated::Punctuated::new();
    bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
        paren_token: None,
        modifier: syn::TraitBoundModifier::None,
        lifetimes: None,
        path: syn::Path {
            leading_colon: None,
            segments
        }
    }));
    syn::GenericArgument::Constraint(syn::Constraint {
        ident: generic_name,
        generics: None,
        colon_token: syn::Token![:](proc_macro2::Span::call_site()),
        bounds
    })
}

fn extract_type(typ: &mut syn::Type, trait_name: &syn::Ident, deref_type: &mut Option<syn::Ident>) -> Result<TypeInfo, TokenStream> {
    match typ {
        syn::Type::Path(ref mut typ) => {
            let ident = match typ.path.segments.first() {
                Some(path) => path.ident.clone(),
                None => return Err(syn::Error::new_spanned(typ, "Type has no path segments").to_compile_error().into()),
            };

            match typ.path.segments.last_mut().expect("To have at least on type path segment").arguments {
                syn::PathArguments::AngleBracketed(ref mut args) => {
                    let result = args.clone();

                    for arg in args.args.iter_mut() {
                        if let syn::GenericArgument::Constraint(constraint) = arg {

                            for param in constraint.bounds.iter() {
                                if let syn::TypeParamBound::Trait(bound) = param {
                                    if bound.path.is_ident(trait_name) {
                                        if let Some(ident) = deref_type.replace(constraint.ident.clone()) {
                                            return Err(syn::Error::new_spanned(ident, "Multiple bounds to trait, can be problematic so how about no?").to_compile_error().into());
                                        }
                                    }
                                }
                            }

                            let mut segments = syn::punctuated::Punctuated::new();
                            segments.push(syn::PathSegment {
                                ident: constraint.ident.clone(),
                                arguments: syn::PathArguments::None
                            });

                            *arg = syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                                qself: None,
                                path: syn::Path {
                                    leading_colon: None,
                                    segments
                                },
                            }));
                        }
                    }

                    //if deref_type.is_none() && result.args.len() == 1 {
                    //    result.args.last_mut();
                    //}

                    Ok(TypeInfo {
                        ident,
                        generics: Some(result),
                        reference: None,
                        mutability: None,
                    })
                },
                syn::PathArguments::None => Ok(TypeInfo {
                    ident,
                    generics: None,
                    reference: None,
                    mutability: None,
                }),
                syn::PathArguments::Parenthesized(ref args) => Err(syn::Error::new_spanned(args, "Unsupported type arguments").to_compile_error().into()),
            }
        },
        syn::Type::Reference(reference) => match extract_type(&mut reference.elem, trait_name, deref_type) {
            Ok(mut result) => {
                result.mutability = reference.mutability;
                result.reference = reference.lifetime.clone();
                Ok(result)
            },
            Err(error) => Err(error),
        }
        other => Err(syn::Error::new_spanned(other, "Unsupported type").to_compile_error().into()),
    }
}

///Generates trait implementation for specified type, relying on `Deref` or `Into` depending on
///whether `self` is reference or owned
///
///Note that this crate is only needed due to lack of specialization that would allow to have
///generic implementation over `T: Deref<Target=O>`
///
///## Example
///
///```rust
///use auto_trait::auto_trait;
///pub struct Wrapper(u32);
///
///impl Into<u32> for Wrapper {
///    fn into(self) -> u32 {
///        self.0
///    }
///}
///
///impl core::ops::Deref for Wrapper {
///    type Target = u32;
///    fn deref(&self) -> &Self::Target {
///        &self.0
///    }
///}
///
///impl core::ops::DerefMut for Wrapper {
///    fn deref_mut(&mut self) -> &mut Self::Target {
///        &mut self.0
///    }
///}
///
///#[auto_trait(Wrapper)]
///pub trait Lolka3 {
///}
///
///impl Lolka3 for u32 {}
///
///#[auto_trait(Box<T: Lolka2>)]
///#[auto_trait(Wrapper)]
///#[auto_trait(&'a mut R)]
///pub trait Lolka2 {
///   fn lolka2_ref(&self) -> u32;
///   fn lolka2_mut(&mut self) -> u32;
///}
///
///impl Lolka2 for u32 {
///   fn lolka2_ref(&self) -> u32 {
///       10
///   }
///   fn lolka2_mut(&mut self) -> u32 {
///       11
///   }
///}
///
///#[auto_trait(Box<T: Lolka + From<Box<T>>>)]
///pub trait Lolka {
///   fn lolka() -> u32;
///
///   fn lolka_ref(&self) -> u32;
///
///   fn lolka_mut(&mut self) -> u32;
///
///   fn lolka_self(self) -> u32;
///}
///
///impl Lolka for u32 {
///   fn lolka() -> u32 {
///       1
///   }
///
///   fn lolka_ref(&self) -> u32 {
///       2
///   }
///
///   fn lolka_mut(&mut self) -> u32 {
///       3
///   }
///
///   fn lolka_self(self) -> u32 {
///       4
///   }
///
///}
///
///let mut lolka = 0u32;
///let mut wrapped = Box::new(lolka);
///
///assert_eq!(lolka.lolka_ref(), wrapped.lolka_ref());
///assert_eq!(lolka.lolka_mut(), wrapped.lolka_mut());
///assert_eq!(lolka.lolka_self(), wrapped.lolka_self());
///
///assert_eq!(lolka.lolka2_ref(), wrapped.lolka2_ref());
///assert_eq!(lolka.lolka2_mut(), wrapped.lolka2_mut());
///
///assert_eq!(lolka.lolka2_ref(), (&mut lolka).lolka2_ref());
///assert_eq!(lolka.lolka2_mut(), (&mut lolka).lolka2_mut());
///```
#[proc_macro_attribute]
pub fn auto_trait(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = syn::parse_macro_input!(input as syn::ItemTrait);
    let args: syn::Type = match syn::parse(args) {
        Ok(args) => args,
        Err(error) => {
            return syn::Error::new(error.span(), "Argument is required and must be a type").to_compile_error().into()
        }
    };

    let mut args = vec![args];

    //We need to remove attributes that we're going to parse
    let mut remaining_attrs = Vec::new();
    for attr in input.attrs.drain(..) {
        if attr.path().is_ident("auto_trait") {
            match attr.parse_args() {
                Ok(arg) => match arg {
                    syn::Type::Paren(arg) => args.push(*arg.elem),
                    arg => args.push(arg),
                },
                Err(error) => {
                    return syn::Error::new(error.span(), "Argument is required and must be a type").to_compile_error().into()
                }
            }
        } else {
            remaining_attrs.push(attr)
        }
    }
    input.attrs = remaining_attrs;

    let mut impls = Vec::new();

    for mut args in args.drain(..) {
        let trait_name = input.ident.clone();
        let mut deref_type = None;
        let type_info = match extract_type(&mut args, &trait_name, &mut deref_type) {
            Ok(type_info) => type_info,
            Err(error) => return error,
        };

        let deref_name = deref_type.unwrap_or_else(|| trait_name.clone());

        let mut methods = Vec::new();

        for item in input.items.iter() {
            match item {
                syn::TraitItem::Fn(ref method) => {
                    let method_name = method.sig.ident.clone();
                    let mut method_args = Vec::new();
                    for arg in method.sig.inputs.iter() {
                        match arg {
                            syn::FnArg::Receiver(arg) => {
                                if arg.reference.is_some() {
                                    if arg.mutability.is_some() {
                                        if type_info.reference.is_some() {
                                            method_args.push(quote! {
                                                &mut **self
                                            })
                                        } else {
                                            method_args.push(quote! {
                                                core::ops::DerefMut::deref_mut(self)
                                            })
                                        }
                                    } else {
                                        if type_info.reference.is_some() {
                                            method_args.push(quote! {
                                                &**self
                                            })
                                        } else {
                                            method_args.push(quote! {
                                                core::ops::Deref::deref(self)
                                            })
                                        }
                                    }
                                } else {
                                    method_args.push(quote! {
                                        self.into()
                                    })
                                }
                            },
                            syn::FnArg::Typed(arg) => {
                                let name = &arg.pat;
                                method_args.push(quote! {
                                    #name
                                })
                            },
                        }
                    }

                    let deref_block: syn::Block = syn::parse2(quote! {
                        {
                            #deref_name::#method_name(#(#method_args,)*)
                        }
                    }).unwrap();

                    let mut method = method.clone();
                    method.default = Some(deref_block);
                    method.semi_token = None;

                    methods.push(method);
                },
                unsupported => return syn::Error::new_spanned(unsupported, "Trait contains non-method definitions which is unsupported").to_compile_error().into(),

            }
        }

        let type_generics = if let Some(lifetime) = type_info.reference {
            match type_info.generics {
                Some(mut generics) => {
                    let mut new_args = syn::punctuated::Punctuated::new();
                    new_args.insert(0, generate_self_trait_bound(type_info.ident, &trait_name));
                    new_args.insert(0, syn::GenericArgument::Lifetime(lifetime));
                    while let Some(arg) = generics.args.pop() {
                        new_args.push(arg.into_tuple().0);
                    }
                    generics.args = new_args;
                    Some(generics)
                },
                None => {
                    let mut args = syn::punctuated::Punctuated::new();
                    args.push(syn::GenericArgument::Lifetime(lifetime));
                    args.push(generate_self_trait_bound(type_info.ident, &trait_name));

                    Some(syn::AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: syn::Token![<](proc_macro2::Span::call_site()),
                        args,
                        gt_token: syn::Token![>](proc_macro2::Span::call_site()),
                    })
                }
            }
        } else {
            type_info.generics
        };

        impls.push(quote! {
            impl#type_generics #trait_name for #args {
                #(
                    #methods
                )*
            }
        });
    }

    let mut result = quote! {
        #input
    };
    result.extend(impls.drain(..));

    result.into()
}
