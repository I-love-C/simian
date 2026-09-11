use dashmap::DashMap;
use ra_ap_syntax::ast::{HasModuleItem, HasName};
use ra_ap_syntax::{AstNode, SourceFile, ast};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::*;
use walkdir::WalkDir;

struct Version {
    file_path: PathBuf,
    body: String,
}

// gets an overview of function stats in a rust project and generates a diff directory for manual
// inspection of same-name functions across a codebase
//
// accumulates same-name functions and collects versions,
// versions -> files in a "diff" dir of the function name, found in the main "diff" dir
fn main() {
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

        let items = SourceFile::parse(&src, ra_ap_syntax::Edition::Edition2021)
            .tree()
            .items();

        for item in items {
            match item {
                ast::Item::Fn(f) => {
                    let name = f.name().map(|n| n.text().to_string()).unwrap_or_default();
                    let body = f.syntax().text().to_string();

                    free_floating_functions_count.fetch_add(1, Ordering::Relaxed);
                    function_map.entry(name).or_default().push(Version {
                        file_path: path.clone(),
                        body: body,
                    });
                }
                ast::Item::Impl(imp) => {
                    let Some(items) = imp.assoc_item_list() else {
                        continue;
                    };
                    for assoc in items.assoc_items() {
                        if let ast::AssocItem::Fn(f) = assoc {
                            let has_receiver =
                                f.param_list().and_then(|pl| pl.self_param()).is_some();
                            if has_receiver {
                                impl_functions_count.fetch_add(1, Ordering::Relaxed);
                            } else {
                                associated_functions_count.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    });

    const DIFF_DIF: &str = "diff";
    let _ = std::fs::remove_dir_all(DIFF_DIF); // fails on first run
    _ = std::fs::create_dir(DIFF_DIF)
        .map_err(|e| eprintln!("Cannot create diff dir, reason : {e}"));

    let mut paths_to_format = Vec::<String>::new();
    for (name, body_list) in function_map {
        // only duplicates
        if body_list.len() > 1 {
            let version_dir = format!("{DIFF_DIF}/{name}");
            let _ = std::fs::create_dir_all(&version_dir);
            for (index, version) in body_list.iter().enumerate() {
                let Version { file_path, body } = version;
                let path = format!("{version_dir}/{index}.rs");
                let file_path = file_path.display();
                let content = format!("// function found in {file_path}\n\n{body}");
                _ = std::fs::write(path.clone(), content)
                    .map_err(|e| eprintln!("Couldn't write to file {path}, reason : {e}"));
                paths_to_format.push(path);
            }
        }
    }

    let free_count = free_floating_functions_count.load(Ordering::Relaxed);
    let impl_count = impl_functions_count.load(Ordering::Relaxed);
    let associated_count = associated_functions_count.load(Ordering::Relaxed);

    println!("Number of free floating functions {free_count}");
    println!("Number of impl functions {impl_count}");
    println!("Number of associated functions {associated_count}");

    let total = free_count + impl_count + associated_count;
    println!("Total number of functions {total}");
}
