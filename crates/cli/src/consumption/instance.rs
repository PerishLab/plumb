pub(crate) fn run(declared: plumb::instance::Arguments, json: bool) -> i32 {
    match plumb::instance::select(declared.instance.as_deref()) {
        Ok(instance) => show(&instance, json),
        Err(error) => {
            eprintln!("plumb: {error}");
            1
        }
    }
}

fn show(instance: &plumb::instance::Instance, json: bool) -> i32 {
    if !json {
        match &instance.source.path {
            Some(path) => println!(
                "{}\nsource: {} {}",
                instance.id,
                instance.source.kind,
                path.display()
            ),
            None => println!("{}\nsource: {}", instance.id, instance.source.kind),
        }
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
