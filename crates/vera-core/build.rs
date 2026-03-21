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
}
