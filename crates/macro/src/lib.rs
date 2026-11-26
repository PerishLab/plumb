use proc_macro::TokenStream;

mod catalogue;
use syn::{LitStr, parse_macro_input};

mod prefix;
mod resource;

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

#[doc = r#"Adds prefix() using an explicit canonical uppercase literal (`[A-Z][A-Z0-9_]*`) or, when omitted, the defining package name. Combine with Radicula Cascade and pass prefix() and plumb::config::value (or plumb::config::migrated) explicitly to resolve or layered.

```compile_fail
#[plumb_macro::prefix("")]
struct Config;
```

```compile_fail
#[plumb_macro::prefix("folio")]
struct Config;
```

```compile_fail
#[plumb_macro::prefix("FOLIO-API")]
struct Config;
```

```compile_fail
#[plumb_macro::prefix("1FOLIO")]
struct Config;
```

```compile_fail
#[plumb_macro::prefix("FOLIO API")]
struct Config;
```

```compile_fail
#[plumb_macro::prefix(FOLIO)]
struct Config;
```

```compile_fail
#[plumb_macro::prefix("FOLIO", "EXTRA")]
struct Config;
```
"#]
#[proc_macro_attribute]
pub fn prefix(args: TokenStream, input: TokenStream) -> TokenStream {
    prefix::expand(args.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
