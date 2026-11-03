use serde_json::Value;
use std::fs;
use std::path::PathBuf;

mod pull;
mod recovery;
mod store;

struct Store(PathBuf);

fn truth(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().unwrap() != 0.,
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

fn stop(message: &str, code: i32) -> ! {
    eprintln!("{message}");
    std::process::exit(code);
}

fn main() {
    let mut arguments = std::env::args();
    arguments.next().unwrap();
    let mode = arguments.next().unwrap();
    assert!(mode == "basic" || mode == "recovery");
    let store = Store(PathBuf::from(arguments.next().unwrap()));
    let args: Vec<String> = arguments.collect();
    let recovering = mode == "recovery";
    if recovering
        && args.starts_with(&["pr".into(), "merge".into()])
        && store.0.join("mergefault").exists()
    {
        stop("fixture reached verified merge boundary", 1);
    }
    if !recovering {
        assert_eq!(args[0], "api");
    }
    let payload = args
        .iter()
        .position(|arg| arg == "--input")
        .map(|index| serde_json::from_slice::<Value>(&fs::read(&args[index + 1]).unwrap()).unwrap())
        .unwrap_or(Value::Null);
    store.record(&args, &payload);
    let result = if recovering && args.starts_with(&["pr".into(), "view".into()]) {
        store.ready()
    } else if recovering && args.starts_with(&["pr".into(), "merge".into()]) {
        store.merge(&args)
    } else if recovering {
        store.recovery(&args[1], &payload)
    } else {
        store.basic(&args[1], &payload)
    };
    println!("{result}");
}
