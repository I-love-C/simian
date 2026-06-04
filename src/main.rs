use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use syn::{Item, ItemStruct, Signature};
use walkdir::WalkDir;

// accumulates same-name functions and collects versions,
// versions -> files in a "diff" dir of the function name, found in the main "diff" dir

struct Version {
    file_path: String,
    body: String,
}
// want to tell if there are any usr-defined structs passed in the arg list
// get all structs defined, then filter the lists

fn main() -> anyhow::Result<()> {
    let dir = std::env::args().nth(1).expect("path to project needed");
    let mut function_map = HashMap::<String, Vec<Version>>::new();
    let mut free_floating_functions_count = 0;
    let mut impl_functions_count = 0;
    let mut associated_functions_count = 0;

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != "target") // skips garbage
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(path)?;
        let Ok(ast) = syn::parse_file(&src) else {
            // invalid rust ignored
            continue;
        };
        for item in ast.items {
            if let syn::Item::Fn(f) = item {
                let name = f.sig.ident.to_string();
                let body = quote::quote!(#f).to_string();
                let file_path = path.display().to_string();
                free_floating_functions_count += 1;
                function_map
                    .entry(name)
                    .or_default()
                    .push(Version { file_path, body });
            } else if let syn::Item::Impl(f) = item {
                for impl_item in f.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if f.sig.receiver().is_some() {
                            impl_functions_count += 1;
                        } else {
                            associated_functions_count += 1;
                        }
                    }
                }
            }
        }
    }

    const DIFF_DIF: &str = "diff";
    let _ = std::fs::remove_dir_all(DIFF_DIF); // fails on first run
    std::fs::create_dir(DIFF_DIF)?;

    for (name, body_list) in &function_map {
        // only duplicates
        if body_list.len() > 1 {
            let version_dir = format!("{DIFF_DIF}/{name}");
            let _ = std::fs::create_dir_all(&version_dir);
            for (index, version) in body_list.iter().enumerate() {
                let Version { file_path, body } = version;
                let path = format!("{version_dir}/{index}.rs");
                let content = format!("// function found in {file_path}\n\n{body}");

                std::fs::write(&path, content)?;
                Command::new("rustfmt").arg(&path).status()?;
            }
        }
    }

    println!("Number of free floating functions {free_floating_functions_count}");
    println!("Number of impl functions {impl_functions_count}");
    println!("Number of associated functions {associated_functions_count}");

    let total = free_floating_functions_count + impl_functions_count + associated_functions_count;
    println!("Total number of functions {total}");
    Ok(())
}
