fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("-V") => println!("plumb v{}", env!("CARGO_PKG_VERSION")),
        _ => {
            println!("Usage: plumb --version");
            println!();
            println!("A plumb line for repositories: hold a repo against the skeleton");
            println!("and report where it hangs untrue. Checking lands before scaffolding.");
        }
    }
}
