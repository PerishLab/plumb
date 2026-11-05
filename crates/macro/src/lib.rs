use proc_macro::TokenStream;

mod catalogue;
use syn::{ItemFn, LitStr, parse_macro_input};

mod prefix;
mod resource;
mod trace;

#[proc_macro]
pub fn catalogue(input: TokenStream) -> TokenStream {
    catalogue::expand(input.into(), proc_macro2::Span::call_site())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn resource(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    resource::expand(path, quote::quote!(include_str))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[doc = r#"Carries a crate-relative resource as bytes, with the anchoring law of `resource!`.

Paths are nonempty slash-separated components and cannot contain `.` or `..`.
Absolute paths, backslashes and empty components are refused.

```compile_fail
let _ = plumb_macro::bytes!("");
```

```compile_fail
let _ = plumb_macro::bytes!("/held");
```

```compile_fail
let _ = plumb_macro::bytes!("\\held");
```

```compile_fail
let _ = plumb_macro::bytes!("held\\file");
```

```compile_fail
let _ = plumb_macro::bytes!(".");
```

```compile_fail
let _ = plumb_macro::bytes!("..");
```

```compile_fail
let _ = plumb_macro::bytes!("./tests/held/kept.txt");
```

```compile_fail
let _ = plumb_macro::bytes!("../macro/tests/held/kept.txt");
```

```compile_fail
let _ = plumb_macro::bytes!("tests/held/../held/kept.txt");
```

```compile_fail
let _ = plumb_macro::bytes!("tests/./held/kept.txt");
```

```compile_fail
let _ = plumb_macro::bytes!("tests//held/kept.txt");
```

```compile_fail
let _ = plumb_macro::bytes!("held/");
```

"#]
#[proc_macro]
pub fn bytes(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    resource::expand(path, quote::quote!(include_bytes))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
#[doc = "Wraps ordinary synchronous functions in trace::run and async bodies in trace::carry. Generated paths use the canonical plumb dependency name and require no direct Root import. Const, unsafe, extern, variadic and track_caller signatures are refused. Async context is captured when the function body starts; carry the outer future explicitly to propagate its caller's context."]
pub fn span(args: TokenStream, input: TokenStream) -> TokenStream {
    let name = parse_macro_input!(args as LitStr);
    let item = parse_macro_input!(input as ItemFn);
    trace::expand(name, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[doc = "Adds prefix() using the configuration type's defining package name. Combine with Radicula Cascade and pass prefix() and plumb::config::value explicitly to resolve or layered. This preserves existing application variable names; it declares no CLI identity."]
#[proc_macro_attribute]
pub fn prefix(args: TokenStream, input: TokenStream) -> TokenStream {
    prefix::expand(args.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
