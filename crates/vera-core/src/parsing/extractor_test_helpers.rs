#[cfg(test)]
mod test_helpers {
    use crate::parsing::languages::tree_sitter_grammar;
    use crate::types::Language;

    pub fn print_sexp(source: &str, lang: Language) {
        let grammar = tree_sitter_grammar(lang).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar).unwrap();
        let tree = parser.parse(source, None).unwrap();
        println!("SEXP for {:?}: {}", lang, tree.root_node().to_sexp());
    }
}
