//! Game SDK derives for portable, named world data.

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as Tokens};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Member, parse_macro_input, spanned::Spanned};

/// Declare portable world data.
#[proc_macro_derive(Data)]
pub fn data(input: TokenStream) -> TokenStream {
    derive_data(input, None)
}

/// Declare data attached to an entity.
#[proc_macro_derive(Component)]
pub fn component(input: TokenStream) -> TokenStream {
    derive_data(input, Some("Component"))
}

/// Declare data shared across rules.
#[proc_macro_derive(Resource)]
pub fn resource(input: TokenStream) -> TokenStream {
    derive_data(input, Some("Resource"))
}

/// Declare data sent to later rules of a tick.
#[proc_macro_derive(Event)]
pub fn event(input: TokenStream) -> TokenStream {
    derive_data(input, Some("Event"))
}

/// Declare the data a scene is entered with.
#[proc_macro_derive(Scene)]
pub fn scene(input: TokenStream) -> TokenStream {
    derive_data(input, Some("SceneType"))
}

/// Declare the phases of a scene.
#[proc_macro_derive(Phase)]
pub fn phase(input: TokenStream) -> TokenStream {
    derive_data(input, Some("PhaseType"))
}

fn derive_data(input: TokenStream, marker: Option<&str>) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_data(&input, marker)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_data(input: &DeriveInput, marker: Option<&str>) -> syn::Result<Tokens> {
    let name = &input.ident;
    let mut generics = input.generics.clone();
    let (write, read, declaration, variants) = match &input.data {
        Data::Struct(value) => {
            let fields = &value.fields;
            let members = members(fields);
            let types = types(fields);
            let construct = construct(
                quote!(Self),
                fields,
                &members,
                types
                    .iter()
                    .map(|ty| quote!(<#ty as ::thrum::Data>::read(reader)?)),
            );
            let variants = vec![(quote!(Self), fields)];
            (
                quote!(#(::thrum::Data::write(&self.#members, writer);)*),
                construct,
                quote!(#fields),
                variants,
            )
        }
        Data::Enum(value) => {
            let mut write = Tokens::new();
            let mut read = Tokens::new();
            let mut declaration = Tokens::new();
            let mut variants = Vec::new();
            for (tag, variant) in (0_u32..).zip(&value.variants) {
                let variant_name = &variant.ident;
                let fields = &variant.fields;
                let members = members(fields);
                let types = types(fields);
                let bindings = bindings(fields);
                let path = quote!(Self::#variant_name);
                let pattern = construct(
                    path.clone(),
                    fields,
                    &members,
                    bindings.iter().map(|id| quote!(#id)),
                );
                let decoded = construct(
                    path.clone(),
                    fields,
                    &members,
                    types
                        .iter()
                        .map(|ty| quote!(<#ty as ::thrum::Data>::read(reader)?)),
                );
                write.extend(quote!(#pattern => {
                    ::thrum::Data::write(&#tag, writer);
                    #(::thrum::Data::write(#bindings, writer);)*
                },));
                read.extend(quote!(#tag => #decoded,));
                declaration.extend(quote!(#variant_name #fields));
                variants.push((path, fields));
            }
            (
                quote!(match self { #write }),
                quote!(match <u32 as ::thrum::Data>::read(reader)? {
                    #read
                    _ => return Err(::thrum::__derive::InvalidData),
                }),
                declaration,
                variants,
            )
        }
        Data::Union(value) => {
            return Err(syn::Error::new(
                value.union_token.span(),
                "T0101: unions have no portable world representation; use a struct or enum",
            ));
        }
    };
    let (implementation, same_encoding) = value_traits(name, &input.generics, &variants);
    let field_types = variants.iter().flat_map(|(_, fields)| types(fields));
    for parameter in generics.type_params_mut() {
        parameter.bounds.push(syn::parse_quote!(::thrum::Data));
    }
    let (implementation_generics, use_generics, requirements) = generics.split_for_impl();
    let copy = (marker == Some("Component")).then(|| {
        let mut generics = generics.clone();
        for parameter in generics.type_params_mut() {
            parameter
                .bounds
                .push(syn::parse_quote!(::core::marker::Copy));
        }
        let (parameters, args, requirements) = generics.split_for_impl();
        quote!(impl #parameters ::core::marker::Copy for #name #args #requirements {})
    });
    let marker = marker.map(|marker| {
        let mut generics = generics.clone();
        if marker == "Component" {
            for parameter in generics.type_params_mut() {
                parameter.bounds.push(syn::parse_quote!(::core::marker::Copy));
            }
        }
        let (implementation_generics, use_generics, requirements) = generics.split_for_impl();
        let marker = format_ident!("{marker}");
        quote!(impl #implementation_generics ::thrum::#marker for #name #use_generics #requirements {})
    });
    Ok(quote! {
        #implementation
        #copy
        impl #implementation_generics ::thrum::Data for #name #use_generics #requirements {
            const SCHEMA: &'static str = concat!(module_path!(), "::", stringify!(#name), stringify!(#declaration));
            const SCHEMA_ID: u128 = ::thrum::__derive::schema_id(
                Self::SCHEMA, &[#(<#field_types as ::thrum::Data>::SCHEMA_ID),*],
            );
            fn same_encoding(&self, other: &Self) -> bool {
                #same_encoding
            }
            fn write(&self, writer: &mut ::thrum::__derive::DataWriter<'_, '_>) {
                #write
            }
            fn read(reader: &mut ::thrum::__derive::DataReader<'_, '_>) -> ::core::result::Result<Self, ::thrum::__derive::InvalidData> {
                Ok(#read)
            }
        }
        #marker
    })
}

fn members(fields: &Fields) -> Vec<Member> {
    fields.members().collect()
}

fn types(fields: &Fields) -> Vec<&syn::Type> {
    fields.iter().map(|field| &field.ty).collect()
}

fn bindings(fields: &Fields) -> Vec<Ident> {
    (0..fields.len())
        .map(|index| format_ident!("field_{index}"))
        .collect()
}

fn construct(
    path: Tokens,
    fields: &Fields,
    members: &[Member],
    values: impl Iterator<Item = Tokens>,
) -> Tokens {
    match fields {
        Fields::Named(_) => quote!(#path { #(#members: #values),* }),
        Fields::Unnamed(_) => quote!(#path(#(#values),*)),
        Fields::Unit => path,
    }
}

fn value_traits(
    name: &Ident,
    generics: &syn::Generics,
    variants: &[(Tokens, &Fields)],
) -> (Tokens, Tokens) {
    let mut generics = generics.clone();
    for parameter in generics.type_params_mut() {
        parameter.bounds.push(syn::parse_quote!(::thrum::Data));
    }
    let (parameters, args, requirements) = generics.split_for_impl();
    let mut cloned = Tokens::new();
    let mut equality = Tokens::new();
    let mut encoding_equality = Tokens::new();
    let mut debug = Tokens::new();
    for (path, fields) in variants {
        let members = members(fields);
        let bindings = bindings(fields);
        let right: Vec<_> = (0..fields.len())
            .map(|index| format_ident!("right_{index}"))
            .collect();
        let pattern = construct(
            path.clone(),
            fields,
            &members,
            bindings.iter().map(|id| quote!(#id)),
        );
        let other = construct(
            path.clone(),
            fields,
            &members,
            right.iter().map(|id| quote!(#id)),
        );
        let new_value = construct(
            path.clone(),
            fields,
            &members,
            bindings
                .iter()
                .map(|id| quote!(::core::clone::Clone::clone(#id))),
        );
        cloned.extend(quote!(#pattern => #new_value,));
        equality.extend(quote!((#pattern, #other) => true #(&& #bindings == #right)*,));
        encoding_equality.extend(quote! {
            (#pattern, #other) => true #(&& ::thrum::Data::same_encoding(#bindings, #right))*,
        });
        let label = path.to_string().rsplit("::").next().map_or_else(
            || name.to_string(),
            |variant| {
                if variant == "Self" {
                    name.to_string()
                } else {
                    variant.trim().to_owned()
                }
            },
        );
        let labels = members.iter().map(|member| quote!(#member).to_string());
        let value = match fields {
            Fields::Named(_) => {
                quote!(formatter.debug_struct(#label)#(.field(#labels, #bindings))*.finish())
            }
            Fields::Unnamed(_) => {
                quote!(formatter.debug_tuple(#label)#(.field(#bindings))*.finish())
            }
            Fields::Unit => quote!(formatter.write_str(#label)),
        };
        debug.extend(quote!(#pattern => #value,));
    }
    let different = (variants.len() > 1).then(|| quote!(_ => false,));
    let implementation = quote! {
        #[automatically_derived]
        impl #parameters ::core::clone::Clone for #name #args #requirements {
            fn clone(&self) -> Self { match self { #cloned } }
        }
        #[automatically_derived]
        impl #parameters ::core::cmp::PartialEq for #name #args #requirements {
            fn eq(&self, other: &Self) -> bool { match (self, other) { #equality #different } }
        }
        #[automatically_derived]
        impl #parameters ::core::fmt::Debug for #name #args #requirements {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self { #debug }
            }
        }
    };
    (
        implementation,
        quote!(match (self, other) { #encoding_equality #different }),
    )
}

/// Bundle named rule parameters into one user-defined parameter.
#[proc_macro_derive(Params)]
pub fn params(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_params(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_params(input: &DeriveInput) -> syn::Result<Tokens> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            name.span(),
            "T0102: Params needs a struct of rule parameters",
        ));
    };
    let Some(lifetime) = input
        .generics
        .lifetimes()
        .next()
        .map(|param| &param.lifetime)
    else {
        return Err(syn::Error::new(
            name.span(),
            "T0102: Params needs the lifetime of its rule invocation",
        ));
    };
    if input.generics.params.len() != 1 {
        return Err(syn::Error::new(
            name.span(),
            "T0102: Params takes one rule lifetime",
        ));
    }
    let members = members(&data.fields);
    let indices: Vec<_> = (0..members.len()).map(syn::Index::from).collect();
    let field_types = types(&data.fields);
    let labels = members.iter().map(|member| quote!(#member).to_string());
    let construct = construct(quote!(#name), &data.fields, &members, field_types.iter().zip(&indices).map(|(ty, index)| {
        quote!(<#ty as ::thrum::__derive::Param>::item(&mut state.#index, context))
    }));
    Ok(quote! {
        #[automatically_derived]
        unsafe impl<#lifetime> ::thrum::__derive::Param for #name<#lifetime> {
            type State = (#(<#field_types as ::thrum::__derive::Param>::State,)*);
            type Item<'__thrum> = #name<'__thrum>;
            fn describe(name: &str, list: &mut ::thrum::__derive::ParameterList) {
                #(<#field_types as ::thrum::__derive::Param>::describe(&format!("{name}.{}", #labels), list);)*
            }
            fn read(state: &mut Self::State, context: &::thrum::__derive::ReadContext<'_>) -> ::thrum::Result {
                #(<#field_types as ::thrum::__derive::Param>::read(&mut state.#indices, context)?;)*
                Ok(())
            }
            fn item<'__thrum>(state: &'__thrum mut Self::State, context: &::thrum::__derive::ReadContext<'_>) -> Self::Item<'__thrum> {
                #construct
            }
            fn journal(state: &mut Self::State, context: &mut ::thrum::__derive::Context<'_, '_>) -> ::thrum::Result {
                let result = Ok(());
                #(let result = result.and(<#field_types as ::thrum::__derive::Param>::journal(&mut state.#indices, context));)*
                result
            }
            fn write(state: &mut Self::State, context: &mut ::thrum::__derive::Context<'_, '_>) -> ::thrum::Result {
                #(<#field_types as ::thrum::__derive::Param>::write(&mut state.#indices, context)?;)*
                Ok(())
            }
        }
        #[automatically_derived]
        impl<#lifetime> ::thrum::__derive::ReadOnly for #name<#lifetime>
        where #(#field_types: ::thrum::__derive::ReadOnly,)*
        {}

    })
}
