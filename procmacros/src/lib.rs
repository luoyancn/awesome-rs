use std::collections::HashMap;

use proc_macro::TokenStream;

use proc_macro2;
use quote::quote;
use syn::visit::{self, Visit};
use syn::{self, parse_macro_input, parse_quote, spanned::Spanned};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let st = parse_macro_input!(input as syn::DeriveInput);
    match do_expand(&st) {
        Ok(token_stream) => token_stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn do_expand(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name_literal = st.ident.to_string();
    let builder_name_literal = format!("{}Builder", struct_name_literal);
    let builder_name_ident = syn::Ident::new(&builder_name_literal, st.span());
    let struct_name_ident = &st.ident;

    let builder_struct_fields_def = generate_builder_struct_fields_def(st)?;
    let builder_struct_fields_init = generate_builder_struct_fields_init(st)?;

    let setter_functions = generate_setter(st)?;
    let checked_res = check_fileds(st)?;
    let build_res = build_target_fields(st)?;

    let ret = quote!(
        pub struct #builder_name_ident {
            #builder_struct_fields_def
        }

        impl #struct_name_ident {
            pub fn builder() -> #builder_name_ident {
                #builder_name_ident {
                    #(#builder_struct_fields_init),*
                }
            }
        }

        impl #builder_name_ident {
            #setter_functions
            pub fn build(&mut self) -> std::result::Result<#struct_name_ident, std::boxed::Box<dyn std::error::Error>>{
                #checked_res
                Ok(#struct_name_ident {
                    #build_res
                })
            }
        }

    );
    Ok(ret)
}

fn check_fileds(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = get_filed_from_derive_input(st)?;
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let mut final_check_stream = proc_macro2::TokenStream::new();

    for (ident, type_) in idents.iter().zip(types.iter()) {
        if get_option_fields(type_).is_some() {
            continue;
        }
        let check_stream_slice = quote! {
            if self.#ident.is_none() {
                let err_msg = format!("{} field is missing", stringify!(#ident));
                return std::result::Result::Err(err_msg.into());
            }
        };
        final_check_stream.extend(check_stream_slice);
    }
    Ok(final_check_stream)
}

fn build_target_fields(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = get_filed_from_derive_input(st)?;
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let mut final_init_stream = proc_macro2::TokenStream::new();
    for (ident, types_) in idents.iter().zip(types.iter()) {
        let init_stream_slice = if get_option_fields(types_).is_none() {
            quote! {
                #ident: self.#ident.clone().unwrap(),
            }
        } else {
            quote! {
                #ident: self.#ident.clone(),
            }
        };
        final_init_stream.extend(init_stream_slice);
    }
    Ok(final_init_stream)
}

fn generate_setter(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = get_filed_from_derive_input(st)?;
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    let mut final_token_stream = proc_macro2::TokenStream::new();
    for (ident, type_) in idents.iter().zip(types.iter()) {
        let token_stream_slice = if let Some(inner_type) = get_option_fields(type_) {
            quote! {
                pub fn #ident(&mut self, #ident: #inner_type) -> & mut Self {
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
            }
        } else {
            quote! {
                pub fn #ident(&mut self, #ident: #type_) -> & mut Self {
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
            }
        };
        final_token_stream.extend(token_stream_slice);
    }
    Ok(final_token_stream)
}

type StructFields = syn::punctuated::Punctuated<syn::Field, syn::Token![,]>;

fn get_filed_from_derive_input(st: &syn::DeriveInput) -> syn::Result<&StructFields> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = st.data
    {
        return Ok(named);
    }
    Err(syn::Error::new_spanned(
        st,
        "Must define on Struct, Not on Enum",
    ))
}

fn generate_builder_struct_fields_def(
    st: &syn::DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let fields = get_filed_from_derive_input(st)?;
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields
        .iter()
        .map(|f| {
            if let Some(ty) = get_option_fields(&f.ty) {
                ty
            } else {
                &f.ty
            }
        })
        .collect();

    let ret = quote! {
        #(#idents: std::option::Option<#types>), *
    };

    Ok(ret)
}

fn get_option_fields(st: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = st
    {
        if let Some(segment) = segments.last() {
            if segment.ident.to_string() == "Option" {
                {
                    if let syn::PathArguments::AngleBracketed(
                        syn::AngleBracketedGenericArguments { ref args, .. },
                    ) = segment.arguments
                    {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.first() {
                            return Some(inner_type);
                        }
                    }
                }
            }
        }
    }
    None
}

fn generate_builder_struct_fields_init(
    st: &syn::DeriveInput,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let fields = get_filed_from_derive_input(st)?;
    let init_data: Vec<_> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            quote! {
                #ident: std::option::Option::None
            }
        })
        .collect();
    Ok(init_data)
}

/***************************************************************************************************/

#[proc_macro_derive(BuilderEach, attributes(builder))]
pub fn deriveach(input: TokenStream) -> TokenStream {
    let st = parse_macro_input!(input as syn::DeriveInput);
    match do_expand_each(&st) {
        Ok(token_stream) => token_stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn do_expand_each(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name_literal = st.ident.to_string();
    let builder_name_literal = format!("{}Builder", struct_name_literal);
    let builder_name_ident = syn::Ident::new(&builder_name_literal, st.span());
    let struct_name_ident = &st.ident;

    let fields = get_filed_from_derive_input_each(st)?;
    let builder_struct_fields_def = generate_builder_struct_fields_def_each(fields)?;
    let builder_struct_fields_init = generate_builder_struct_fields_init_each(fields)?;

    let setter_functions = generate_setter_each(fields)?;
    let build_function = generate_builder_function(fields, &struct_name_ident)?;

    let ret = quote!(
        pub struct #builder_name_ident {
            #builder_struct_fields_def
        }

        impl #struct_name_ident {
            pub fn builder() -> #builder_name_ident {
                #builder_name_ident {
                    #(#builder_struct_fields_init),*
                }
            }
        }

        impl #builder_name_ident {
            #setter_functions
            #build_function
        }

    );
    Ok(ret)
}

fn get_filed_from_derive_input_each(st: &syn::DeriveInput) -> syn::Result<&StructFields> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = st.data
    {
        return Ok(named);
    }
    Err(syn::Error::new_spanned(
        st,
        "Must define on Struct, Not on Enum",
    ))
}

fn get_attr_field_ident(field: &syn::Field) -> syn::Result<Option<syn::Ident>> {
    for attr in &field.attrs {
        if let Ok(syn::Meta::List(ref list)) = attr.parse_meta() {
            let syn::MetaList {
                ref path,
                ref nested,
                ..
            } = list;

            if let Some(__path__) = path.segments.first() {
                if __path__.ident == "builder" {
                    if let Some(syn::NestedMeta::Meta(syn::Meta::NameValue(__dict__))) =
                        nested.first()
                    {
                        if __dict__.path.is_ident("each") {
                            if let syn::Lit::Str(ref arg_token) = __dict__.lit {
                                return Ok(Some(syn::Ident::new(
                                    arg_token.value().as_str(),
                                    attr.span(),
                                )));
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                list,
                                r#"expected `builder(each = "...")`"#,
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn get_generic_fields_type_each<'a>(
    st: &'a syn::Type,
    outer_ident_name: &str,
) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = st
    {
        if let Some(segment) = segments.last() {
            if segment.ident.to_string() == outer_ident_name {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) = &segment.arguments
                {
                    if let Some(syn::GenericArgument::Type(inner_type)) = args.first() {
                        return Some(inner_type);
                    }
                }
            }
        }
    }
    None
}

fn generate_builder_struct_fields_def_each(
    fields: &StructFields,
) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: syn::Result<Vec<_>> = fields
        .iter()
        .map(|f| {
            if let Some(inner_type) = get_generic_fields_type_each(&f.ty, "Option") {
                Ok(quote!(std::option::Option<#inner_type>))
            } else if get_attr_field_ident(f)?.is_some() {
                let origin_type = &f.ty;
                Ok(quote!(#origin_type))
            } else {
                let origin_type = &f.ty;
                Ok(quote!(std::option::Option<#origin_type>))
            }
        })
        .collect();

    let __types__ = types?;
    Ok(quote!( #(#idents: #__types__), *))
}

fn generate_builder_struct_fields_init_each(
    fields: &StructFields,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let init_data: syn::Result<Vec<proc_macro2::TokenStream>> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            if get_attr_field_ident(f)?.is_some() {
                Ok(quote!(#ident: std::vec::Vec::new()))
            } else {
                Ok(quote!(#ident: std::option::Option::None))
            }
        })
        .collect();
    Ok(init_data?)
}

fn generate_setter_each(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    let mut final_token_stream = proc_macro2::TokenStream::new();
    for (idx, (ident, type_)) in idents.iter().zip(types.iter()).enumerate() {
        let mut tokenstream_piece;
        if let Some(inner_type) = get_generic_fields_type_each(type_, "Option") {
            tokenstream_piece = quote! {
                pub fn #ident(&mut self, #ident: #inner_type) -> & mut Self {
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
            };
        } else if let Some(ref builder_for_each) = get_attr_field_ident(&fields[idx])? {
            let inner_type = get_generic_fields_type_each(type_, "Vec").ok_or(syn::Error::new(
                fields[idx].span(),
                "each field must be specified with Vec field",
            ))?;
            tokenstream_piece = quote! {
                pub fn #builder_for_each(&mut self, #builder_for_each: #inner_type) -> & mut Self {
                    self.#ident.push(#builder_for_each);
                    self
                }
            };

            if builder_for_each != ident.as_ref().unwrap() {
                tokenstream_piece.extend(quote! {
                    pub fn #ident(&mut self, #ident: #type_) -> & mut Self {
                        self.#ident = #ident.clone();
                        self
                    }
                });
            }
        } else {
            tokenstream_piece = quote! {
                pub fn #ident(&mut self, #ident: #type_) -> & mut Self {
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
            };
        };
        final_token_stream.extend(tokenstream_piece);
    }
    Ok(final_token_stream)
}

fn generate_builder_function(
    fields: &StructFields,
    origin_struct_ident: &syn::Ident,
) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    let mut check_pieces = Vec::new();
    for (idx, (__ident__, __type__)) in idents.iter().zip(types.iter()).enumerate() {
        if get_generic_fields_type_each(__type__, "Option").is_none()
            && get_attr_field_ident(&fields[idx])?.is_none()
        {
            check_pieces.push(quote! {
                if self.#__ident__.is_none() {
                    let err = format!("{} field missing", stringify!(#__ident__));
                    return std::result::Result::Err(err.into())
                }
            });
        }
    }

    let mut fill_result = Vec::new();
    for (idx, (__ident__, __type__)) in idents.iter().zip(types.iter()).enumerate() {
        if get_attr_field_ident(&fields[idx])?.is_some() {
            fill_result.push(quote!(#__ident__: self.#__ident__.clone()));
        } else if get_generic_fields_type_each(__type__, "Option").is_none() {
            fill_result.push(quote!(#__ident__: self.#__ident__.clone().unwrap()));
        } else {
            fill_result.push(quote!(#__ident__: self.#__ident__.clone()));
        }
    }

    let final_token = quote! {
        pub fn build(&mut self) -> std::result::Result<#origin_struct_ident, std::boxed::Box<dyn std::error::Error>>{
            #(#check_pieces)*
            Ok(#origin_struct_ident {
                #(#fill_result),*
            })
        }
    };

    Ok(final_token)
}

/***************************************************************************************************/
#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive_debug(input: TokenStream) -> TokenStream {
    let st = parse_macro_input!(input as syn::DeriveInput);
    match do_expand_debug(&st) {
        Ok(token_stream) => token_stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn do_expand_debug(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    Ok(generate_debug_trait(st)?)
}

fn generate_debug_trait(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name_literal = st.ident.to_string();
    let struct_name_ident = &st.ident;
    let mut debug_body = proc_macro2::TokenStream::new();

    debug_body.extend(quote! {
        ft.debug_struct(#struct_name_literal)
    });

    let fields = get_filed_from_derive_input_debug(st)?;
    for field in fields.iter() {
        let ident = field.ident.as_ref().unwrap();
        let ident_iteral = ident.to_string();
        if let Some(format_label) = get_speciald_format_of_field(&field)? {
            debug_body
                .extend(quote! {.field(#ident_iteral, &format_args!(#format_label ,&self.#ident))});
        } else {
            debug_body.extend(quote! {.field(#ident_iteral, &self.#ident)});
        }
    }

    debug_body.extend(quote! {.finish()});

    let generics = get_generics(&st)?;

    let (impl_block, generic_block, where_block) = generics.split_for_impl();

    let ret = quote! {
        impl #impl_block std::fmt::Debug for #struct_name_ident #generic_block #where_block {
            fn fmt(&self, ft: &mut std::fmt::Formatter) -> std::fmt::Result {
                #debug_body
            }
        }
    };

    Ok(ret)
}

fn get_filed_from_derive_input_debug(st: &syn::DeriveInput) -> syn::Result<&StructFields> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = st.data
    {
        return Ok(named);
    }
    Err(syn::Error::new_spanned(
        st,
        "Must define on Struct, Not on Enum",
    ))
}

fn get_speciald_format_of_field(field: &syn::Field) -> syn::Result<Option<String>> {
    for attr in &field.attrs {
        if let Ok(syn::Meta::NameValue(syn::MetaNameValue {
            ref path, ref lit, ..
        })) = attr.parse_meta()
        {
            if path.is_ident("debug") {
                if let syn::Lit::Str(ref ident_str) = lit {
                    let ret = ident_str.value();
                    let len = ret.len();
                    match ret.chars().nth(len - 2) {
                        Some('e') | Some('E') | Some('o') | Some('p') | Some('b') | Some('x')
                        | Some('X') | Some('?') => return Ok(Some(ident_str.value())),
                        _ => {
                            return Err(syn::Error::new(ident_str.span(), "Unexpected format args"))
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn get_generics(st: &syn::DeriveInput) -> syn::Result<syn::Generics> {
    let fields = get_filed_from_derive_input_debug(st)?;
    let mut generics_params = st.generics.clone();
    let mut field_type_name = Vec::new();
    let mut phantomdata_type_param_names = Vec::new();
    for field in fields {
        if let Some(name) = get_field_type_name(&field)? {
            field_type_name.push(name);
        }
        if let Some(phd) = get_phantomdata_generic_type_name(&field)? {
            phantomdata_type_param_names.push(phd);
        }
    }
    let associated_types_map = get_generic_associated_types(st);
    for param in generics_params.params.iter_mut() {
        if let syn::GenericParam::Type(t) = param {
            let type_name = t.ident.to_string();
            if phantomdata_type_param_names.contains(&type_name)
                && !field_type_name.contains(&type_name)
            {
                continue;
            }
            if associated_types_map.contains_key(&type_name)
                && !field_type_name.contains(&type_name)
            {
                continue;
            }
            t.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }
    generics_params.make_where_clause();
    for (_, associated_types) in associated_types_map {
        for associated_type in associated_types {
            generics_params
                .where_clause
                .as_mut()
                .unwrap()
                .predicates
                .push(parse_quote!(#associated_type:std::fmt::Debug));
        }
    }
    Ok(generics_params)
}

fn get_phantomdata_generic_type_name(field: &syn::Field) -> syn::Result<Option<String>> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { ref segments, .. },
        ..
    }) = field.ty
    {
        if let Some(syn::PathSegment {
            ref ident,
            ref arguments,
        }) = segments.last()
        {
            if ident == "PhantomData" {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) = arguments
                {
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(ref arg_path))) =
                        args.first()
                    {
                        if let Some(generic_ident) = arg_path.path.segments.first() {
                            return Ok(Some(generic_ident.ident.to_string()));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn get_field_type_name(field: &syn::Field) -> syn::Result<Option<String>> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { ref segments, .. },
        ..
    }) = field.ty
    {
        if let Some(syn::PathSegment { ref ident, .. }) = segments.last() {
            return Ok(Some(ident.to_string()));
        }
    }
    Ok(None)
}

struct TypePathVisitor {
    generic_type_names: Vec<String>,
    associated_types: HashMap<String, Vec<syn::TypePath>>,
}

impl<'ast> Visit<'ast> for TypePathVisitor {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if node.path.segments.len() >= 2 {
            let generic_type_name = node.path.segments[0].ident.to_string();
            if self.generic_type_names.contains(&generic_type_name) {
                self.associated_types
                    .entry(generic_type_name)
                    .or_insert(Vec::new())
                    .push(node.clone());
            }
        }
        visit::visit_type_path(self, node);
    }
}

fn get_generic_associated_types(st: &syn::DeriveInput) -> HashMap<String, Vec<syn::TypePath>> {
    let origin_generic_param_names: Vec<String> = st
        .generics
        .params
        .iter()
        .filter_map(|f| {
            if let syn::GenericParam::Type(ty) = f {
                return Some(ty.ident.to_string());
            }
            return None;
        })
        .collect();

    let mut visitor = TypePathVisitor {
        generic_type_names: origin_generic_param_names,
        associated_types: HashMap::new(),
    };

    visitor.visit_derive_input(st);
    return visitor.associated_types;
}
