pub(crate) fn run(json: bool) -> i32 {
    match plumb::instance::current() {
        Ok(instance) => show(&instance, json),
        Err(error) => {
            eprintln!("plumb: {error}");
            1
        }
    }
}

fn show(instance: &plumb::instance::Instance, json: bool) -> i32 {
    if !json {
        println!(
            "{}\nsource: {}",
            instance.id,
            instance.source.path.display()
        );
        return 0;
    }
    match serde_json::to_string(instance) {
        Ok(body) => {
            println!("{body}");
            0
        }
        Err(error) => {
            eprintln!("plumb: cannot encode instance: {error}");
            1
        }
    }
}
