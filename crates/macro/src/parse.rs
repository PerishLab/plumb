use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Field, Fields, GenericArgument, Ident, PathArguments, Type,
};

struct Row {
    name: Ident,
    ty: Type,
    section: bool,
    arg: bool,
}

pub fn expand(item: DeriveInput) -> Result<TokenStream, syn::Error> {
    let rows = rows(&item)?;
    let section = marked(&item.attrs)?;
    if section && rows.iter().any(|row| row.arg) {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "a section holds no args, args belong to the app struct",
        ));
    }
    let name = &item.ident;
    let vis = &item.vis;
    let partial = format_ident!("{}Partial", name);
    let held = rows.iter().map(|row| held(row, vis));
    let reads = rows.iter().map(read);
    let merges = rows.iter().map(merge);
    let mut out = quote! {
        #[derive(Debug, Default, ::plumb::serde::Deserialize)]
        #[serde(crate = "::plumb::serde", default)]
        #vis struct #partial {
            #(#held,)*
        }

        impl ::plumb::config::Cascade for #name {
            type Partial = #partial;
            fn env_with(
                prefix: &str,
                get: &dyn Fn(&str) -> ::core::option::Option<::std::string::String>,
            ) -> ::core::result::Result<#partial, ::plumb::config::Error> {
                ::core::result::Result::Ok(#partial { #(#reads,)* })
            }
            fn merge(mut self, over: #partial) -> Self {
                #(#merges)*
                self
            }
        }
    };
    if !section {
        out.extend(app(&item, &partial));
        out.extend(armed(&item, &partial, &rows));
    }
    Ok(out)
}

fn rows(item: &DeriveInput) -> Result<Vec<Row>, syn::Error> {
    let Data::Struct(body) = &item.data else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "Cascade derives on a named-field struct",
        ));
    };
    let Fields::Named(fields) = &body.fields else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "Cascade derives on a named-field struct",
        ));
    };
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "Cascade derives on a plain struct",
        ));
    }
    fields.named.iter().map(read_field).collect()
}

fn held(row: &Row, vis: &syn::Visibility) -> TokenStream {
    let field = &row.name;
    let ty = &row.ty;
    if row.section {
        quote! { #[serde(default)] #vis #field: <#ty as ::plumb::config::Cascade>::Partial }
    } else {
        quote! { #vis #field: ::core::option::Option<#ty> }
    }
}

fn read(row: &Row) -> TokenStream {
    let field = &row.name;
    let ty = &row.ty;
    let upper = row.name.to_string().to_uppercase();
    if row.section {
        return quote! {
            #field: <#ty as ::plumb::config::Cascade>::env_with(
                &::std::format!("{}_{}", prefix, #upper),
                get,
            )?
        };
    }
    quote! {
        #field: {
            let key = ::std::format!("{}_{}", prefix, #upper);
            #[allow(unused_imports)]
            use ::plumb::config::Opaque as _;
            (&::plumb::config::Sniff::<#ty>(::core::marker::PhantomData)).take(get(&key), &key)?
        }
    }
}

fn merge(row: &Row) -> TokenStream {
    let field = &row.name;
    let ty = &row.ty;
    if row.section {
        return quote! {
            self.#field = <#ty as ::plumb::config::Cascade>::merge(self.#field, over.#field);
        };
    }
    quote! {
        if let ::core::option::Option::Some(value) = over.#field {
            self.#field = value;
        }
    }
}

fn app(item: &DeriveInput, partial: &Ident) -> TokenStream {
    let name = &item.ident;
    let vis = &item.vis;
    quote! {
        impl #name {
            #vis fn prefix() -> ::std::string::String {
                ::std::env!("CARGO_PKG_NAME").to_uppercase().replace('-', "_")
            }
            #vis fn resolve(
                file: ::core::option::Option<&::std::path::Path>,
            ) -> ::core::result::Result<Self, ::plumb::config::Error> {
                Self::resolve_with(file, ::core::default::Default::default())
            }
            #vis fn resolve_with(
                file: ::core::option::Option<&::std::path::Path>,
                over: #partial,
            ) -> ::core::result::Result<Self, ::plumb::config::Error> {
                let mut held = <Self as ::core::default::Default>::default();
                if let ::core::option::Option::Some(path) = file {
                    held = ::plumb::config::Cascade::merge(
                        held,
                        ::plumb::config::load::<#partial>(path)?,
                    );
                }
                let seen = <Self as ::plumb::config::Cascade>::env(&Self::prefix())?;
                held = ::plumb::config::Cascade::merge(held, seen);
                ::core::result::Result::Ok(::plumb::config::Cascade::merge(held, over))
            }
        }
    }
}

fn armed(item: &DeriveInput, partial: &Ident, rows: &[Row]) -> TokenStream {
    let flagged: Vec<&Row> = rows.iter().filter(|row| row.arg).collect();
    if flagged.is_empty() {
        return TokenStream::new();
    }
    let vis = &item.vis;
    let args_name = format_ident!("{}Args", item.ident);
    let fields = flagged.iter().map(|row| arm(row, vis));
    let moves = flagged.iter().map(|row| carry(row));
    quote! {
        #[derive(::clap::Args, Clone, Debug)]
        #vis struct #args_name {
            #(#fields,)*
        }
        impl #args_name {
            #vis fn partial(self) -> #partial {
                #partial { #(#moves,)* ..::core::default::Default::default() }
            }
        }
    }
}

fn arm(row: &Row, vis: &syn::Visibility) -> TokenStream {
    let field = &row.name;
    let long = row.name.to_string().replace('_', "-");
    let ty = bare(&row.ty).unwrap_or(&row.ty);
    quote! { #[arg(long = #long)] #vis #field: ::core::option::Option<#ty> }
}

fn carry(row: &Row) -> TokenStream {
    let field = &row.name;
    if bare(&row.ty).is_some() {
        return quote! { #field: self.#field.map(::core::option::Option::Some) };
    }
    quote! { #field: self.#field }
}

fn marked(attrs: &[Attribute]) -> Result<bool, syn::Error> {
    let mut section = false;
    for attr in attrs {
        if !attr.path().is_ident("cascade") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                section = true;
                return Ok(());
            }
            Err(meta.error("use #[cascade(section)] on the struct"))
        })?;
    }
    Ok(section)
}

fn read_field(field: &Field) -> Result<Row, syn::Error> {
    let mut section = false;
    let mut arg = false;
    for attr in &field.attrs {
        if !attr.path().is_ident("cascade") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                section = true;
                return Ok(());
            }
            if meta.path.is_ident("arg") {
                arg = true;
                return Ok(());
            }
            Err(meta.error("use #[cascade(section)] or #[cascade(arg)]"))
        })?;
    }
    if section && arg {
        return Err(syn::Error::new_spanned(
            field,
            "a section field cannot take #[cascade(arg)]",
        ));
    }
    Ok(Row {
        name: field.ident.clone().expect("fields are named"),
        ty: field.ty.clone(),
        section,
        arg,
    })
}

fn bare(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    if last.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    let GenericArgument::Type(inner) = args.args.first()? else {
        return None;
    };
    Some(inner)
}
