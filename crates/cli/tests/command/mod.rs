mod authority;
mod cloud;
mod depot;
mod identity;
mod registry;

fn product(_: &std::path::Path) -> Result<String, String> {
    unreachable!("registry state tests do not inspect repository shape")
}
