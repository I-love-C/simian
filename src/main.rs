use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::atomic::*;
use std::{process::Command, sync::atomic::AtomicUsize};
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
    let function_map = DashMap::<String, Vec<Version>>::new();
    let free_floating_functions_count = AtomicUsize::new(0);
    let impl_functions_count = AtomicUsize::new(0);
    let associated_functions_count = AtomicUsize::new(0);

    let paths: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != "target")
        .flatten()
        .map(|e| e.path().to_path_buf())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("rs"))
        .collect();

    paths.par_iter().for_each(|path| {
        let Ok(src) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(ast) = syn::parse_file(&src) else {
            return;
        };
        for item in ast.items {
            if let syn::Item::Fn(f) = item {
                let name = f.sig.ident.to_string();
                let body = quote::quote!(#f).to_string();
                let file_path = path.display().to_string();
                free_floating_functions_count.fetch_add(1, Ordering::Relaxed);
                function_map
                    .entry(name)
                    .or_default()
                    .push(Version { file_path, body });
            } else if let syn::Item::Impl(f) = item {
                for impl_item in f.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if f.sig.receiver().is_some() {
                            impl_functions_count.fetch_add(1, Ordering::Relaxed);
                        } else {
                            associated_functions_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        }
    });

    const DIFF_DIF: &str = "diff";
    let _ = std::fs::remove_dir_all(DIFF_DIF); // fails on first run
    std::fs::create_dir(DIFF_DIF)?;

    let mut paths_to_format = Vec::<String>::new();
    for (name, body_list) in function_map {
        // only duplicates
        if body_list.len() > 1 {
            let version_dir = format!("{DIFF_DIF}/{name}");
            let _ = std::fs::create_dir_all(&version_dir);
            for (index, version) in body_list.iter().enumerate() {
                let Version { file_path, body } = version;
                let path = format!("{version_dir}/{index}.rs");
                let content = format!("// function found in {file_path}\n\n{body}");
                std::fs::write(path.clone(), content)?;
                paths_to_format.push(path);
            }
        }
    }

    // formats all files
    Command::new("rustfmt").args(paths_to_format).status()?;

    let free_count = free_floating_functions_count.load(Ordering::Relaxed);
    let impl_count = impl_functions_count.load(Ordering::Relaxed);
    let associated_count = associated_functions_count.load(Ordering::Relaxed);

    println!("Number of free floating functions {free_count}");
    println!("Number of impl functions {impl_count}");
    println!("Number of associated functions {associated_count}");

    let total = free_count + impl_count + associated_count;
    println!("Total number of functions {total}");
    Ok(())
}
