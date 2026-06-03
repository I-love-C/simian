use std::{collections::HashMap, process::Command};

use walkdir::WalkDir;

// accumulates same-name functions and collects versions,
// versions -> files in a "diff" dir of the function name, found in the main "diff" dir

struct Version {
    file: String,
    body: String,
}

fn main() {
    let dir = std::env::args().nth(1).expect("path to project needed");
    let mut function_map = HashMap::<String, Vec<Version>>::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != "target") // skips garbage
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(path).unwrap();
        let Ok(ast) = syn::parse_file(&src) else {
            // invalid rust ignored
            continue;
        };
        for item in ast.items {
            if let syn::Item::Fn(f) = item {
                let name = f.sig.ident.to_string();
                let body = quote::quote!(#f).to_string();

                function_map.entry(name).or_default().push(Version {
                    file: path.display().to_string(),
                    body: body,
                });
            }
        }
    }

    const DIFF_DIF: &str = "diff";
    let _ = std::fs::remove_dir_all(DIFF_DIF);
    std::fs::create_dir(DIFF_DIF).unwrap();

    for (name, body_list) in &function_map {
        // only duplicates
        if body_list.len() > 1 {
            let version_dir = format!("{DIFF_DIF}/{name}");
            std::fs::create_dir_all(&version_dir).unwrap();
            for (index, version) in body_list.iter().enumerate() {
                let file_name = version.file.clone();
                let path = format!("{version_dir}/{index}.rs");

                let body = version.body.clone();
                let content = format!("// function found in {file_name}\n\n{body}");
                std::fs::write(&path, content).unwrap();

                Command::new("rustfmt").arg(&path).status().unwrap();
            }
        }
    }
}
