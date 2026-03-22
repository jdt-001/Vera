fn main() {
    let sql_dir = std::path::Path::new("../tree-sitter-sql/src");
    if sql_dir.exists() {
        println!("cargo:rerun-if-changed=../tree-sitter-sql/src/parser.c");
        println!("cargo:rerun-if-changed=../tree-sitter-sql/src/scanner.cc");
        cc::Build::new()
            .include(sql_dir)
            .file(sql_dir.join("parser.c"))
            .warnings(false)
            .compile("tree-sitter-sql-parser");

        cc::Build::new()
            .include(sql_dir)
            .file(sql_dir.join("scanner.cc"))
            .cpp(true)
            .warnings(false)
            .compile("tree-sitter-sql-scanner");
    }

    let proto_dir = std::path::Path::new("../tree-sitter-proto/src");
    if proto_dir.exists() {
        println!("cargo:rerun-if-changed=../tree-sitter-proto/src/parser.c");
        cc::Build::new()
            .include(proto_dir)
            .file(proto_dir.join("parser.c"))
            .warnings(false)
            .compile("tree-sitter-proto");
    }

    let dockerfile_dir = std::path::Path::new("../tree-sitter-dockerfile/src");
    if !dockerfile_dir.join("parser.c").exists() {
        println!(
            "cargo:warning=tree-sitter-dockerfile grammar not found. Run .factory/init.sh to download."
        );
    }
    if dockerfile_dir.exists() {
        println!("cargo:rerun-if-changed=../tree-sitter-dockerfile/src/parser.c");
        println!("cargo:rerun-if-changed=../tree-sitter-dockerfile/src/scanner.c");
        cc::Build::new()
            .include(dockerfile_dir)
            .file(dockerfile_dir.join("parser.c"))
            .warnings(false)
            .compile("tree-sitter-dockerfile-parser");

        cc::Build::new()
            .include(dockerfile_dir)
            .file(dockerfile_dir.join("scanner.c"))
            .warnings(false)
            .compile("tree-sitter-dockerfile-scanner");
    }

    let vue_dir = std::path::Path::new("../tree-sitter-vue/src");
    if !vue_dir.join("parser.c").exists() {
        println!(
            "cargo:warning=tree-sitter-vue grammar not found. Run .factory/init.sh to download."
        );
    }
    if vue_dir.exists() {
        println!("cargo:rerun-if-changed=../tree-sitter-vue/src/parser.c");
        println!("cargo:rerun-if-changed=../tree-sitter-vue/src/scanner.c");
        cc::Build::new()
            .include(vue_dir)
            .file(vue_dir.join("parser.c"))
            .warnings(false)
            .compile("tree-sitter-vue-parser");

        cc::Build::new()
            .include(vue_dir)
            .file(vue_dir.join("scanner.c"))
            .warnings(false)
            .compile("tree-sitter-vue-scanner");
    }
}
