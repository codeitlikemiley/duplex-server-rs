use glob::glob;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let proto_files: Vec<_> = glob("proto/*.proto")
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .map(|path| path.to_string_lossy().into_owned())
        .collect();

    println!("cargo:warning={:?}", proto_files);

    let mut attributes = Vec::new();

    for proto_file in &proto_files {
        let messages = extract_messages(proto_file);
        for message in messages {
            if message.contains("Request") {
                println!("cargo:warning={:?}", message);
                attributes.push((message, "#[derive(serde::Deserialize)]".to_string()));
            } else if message.contains("Response") {
                println!("cargo:warning={:?}", message);
                attributes.push((message, "#[derive(serde::Serialize)]".to_string()));
            }
        }
    }

    let mut config = tonic_build::configure()
        .out_dir("src/infrastructure/proto")
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path("src/infrastructure/proto/reflection_descriptor.bin");

    for (message, attribute) in attributes {
        config = config.type_attribute(&message, &attribute);
    }

    config
        .compile(&proto_files, &["proto"])
        .unwrap_or_else(|e| panic!("Failed to compile protobuf {:?}", e));
}

fn extract_messages(proto_file: &str) -> Vec<String> {
    let file = File::open(proto_file).expect("Failed to open proto file");
    let reader = BufReader::new(file);

    let message_regex = Regex::new(r"^\s*message\s+(\w+)").expect("Failed to create regex");
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        if let Some(captures) = message_regex.captures(&line) {
            if let Some(message) = captures.get(1) {
                messages.push(message.as_str().to_string());
            }
        }
    }

    messages
}
