# Simian
*Monkey see monkey do...*

Only finds free-floating functions with the same name (params not taken into account), makes a `diff` directory in which each function body is stored as a separate file along with where it was found in a dir of the function name.

```bash
diff
  └── <function_name>
      ├── 0.rs # first  "version"
      └── 1.rs # second "version"
      ...
```

This is to cull functions across a codebase, `new, from, fmt..` also not taken into account.

## Usage
```bash
cargo run <path_to_project_dir>
```
