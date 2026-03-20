//! AST symbol extraction rules per language.
//!
//! Defines which tree-sitter node types correspond to which [`SymbolType`]
//! for each supported language. Walks the AST to extract top-level symbols.

use crate::types::Language;
use crate::types::SymbolType;

/// A raw symbol extracted from the AST before chunking.
#[derive(Debug, Clone)]
pub struct RawSymbol {
    /// Name of the symbol (e.g., function name, class name).
    pub name: Option<String>,
    /// Type of symbol.
    pub symbol_type: SymbolType,
    /// 0-based byte offset of the symbol start in the source.
    pub start_byte: usize,
    /// 0-based byte offset of the symbol end in the source.
    pub end_byte: usize,
    /// 0-based start row in the source.
    pub start_row: usize,
    /// 0-based end row in the source.
    pub end_row: usize,
}

/// Maps a tree-sitter node kind to a [`SymbolType`] for the given language.
///
/// Returns `None` if the node kind is not a top-level symbol we extract.
pub fn classify_node(lang: Language, kind: &str) -> Option<SymbolType> {
    match lang {
        Language::Rust => classify_rust(kind),
        Language::TypeScript | Language::JavaScript => classify_typescript(kind),
        Language::Python => classify_python(kind),
        Language::Go => classify_go(kind),
        Language::Java => classify_java(kind),
        Language::C => classify_c(kind),
        Language::Cpp => classify_cpp(kind),
        _ => None,
    }
}

fn classify_rust(kind: &str) -> Option<SymbolType> {
    match kind {
        "function_item" => Some(SymbolType::Function),
        "impl_item" => Some(SymbolType::Block),
        "struct_item" => Some(SymbolType::Struct),
        "enum_item" => Some(SymbolType::Enum),
        "trait_item" => Some(SymbolType::Trait),
        "type_item" => Some(SymbolType::TypeAlias),
        "const_item" => Some(SymbolType::Constant),
        "static_item" => Some(SymbolType::Constant),
        "mod_item" => Some(SymbolType::Module),
        _ => None,
    }
}

fn classify_typescript(kind: &str) -> Option<SymbolType> {
    match kind {
        "function_declaration" => Some(SymbolType::Function),
        "class_declaration" => Some(SymbolType::Class),
        "interface_declaration" => Some(SymbolType::Interface),
        "type_alias_declaration" => Some(SymbolType::TypeAlias),
        "enum_declaration" => Some(SymbolType::Enum),
        "method_definition" => Some(SymbolType::Method),
        "lexical_declaration" | "variable_declaration" => Some(SymbolType::Variable),
        "export_statement" => None, // recurse into children
        _ => None,
    }
}

fn classify_python(kind: &str) -> Option<SymbolType> {
    match kind {
        "function_definition" => Some(SymbolType::Function),
        "class_definition" => Some(SymbolType::Class),
        "decorated_definition" => None, // recurse into children
        _ => None,
    }
}

fn classify_go(kind: &str) -> Option<SymbolType> {
    match kind {
        "function_declaration" => Some(SymbolType::Function),
        "method_declaration" => Some(SymbolType::Method),
        "type_declaration" => None, // contains type_spec children
        "type_spec" => Some(SymbolType::TypeAlias), // refined by child kind
        _ => None,
    }
}

fn classify_java(kind: &str) -> Option<SymbolType> {
    match kind {
        "method_declaration" => Some(SymbolType::Method),
        "class_declaration" => Some(SymbolType::Class),
        "interface_declaration" => Some(SymbolType::Interface),
        "enum_declaration" => Some(SymbolType::Enum),
        "constructor_declaration" => Some(SymbolType::Method),
        _ => None,
    }
}

fn classify_c(kind: &str) -> Option<SymbolType> {
    match kind {
        "function_definition" => Some(SymbolType::Function),
        "struct_specifier" => Some(SymbolType::Struct),
        "enum_specifier" => Some(SymbolType::Enum),
        "type_definition" => Some(SymbolType::TypeAlias),
        "declaration" => Some(SymbolType::Variable),
        _ => None,
    }
}

fn classify_cpp(kind: &str) -> Option<SymbolType> {
    match kind {
        "function_definition" => Some(SymbolType::Function),
        "class_specifier" => Some(SymbolType::Class),
        "struct_specifier" => Some(SymbolType::Struct),
        "enum_specifier" => Some(SymbolType::Enum),
        "type_definition" => Some(SymbolType::TypeAlias),
        "namespace_definition" => Some(SymbolType::Module),
        "template_declaration" => None, // recurse into children
        "declaration" => Some(SymbolType::Variable),
        _ => None,
    }
}

/// Extract the name of a symbol from a tree-sitter node.
///
/// Looks for the first `name` or `identifier`-type child node.
pub fn extract_name(node: &tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    // Try common name field patterns
    for field in &["name", "declarator"] {
        if let Some(child) = node.child_by_field_name(field) {
            return name_from_node(&child, source);
        }
    }
    // Fallback: look for first identifier child
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "identifier" || kind == "type_identifier" || kind == "property_identifier" {
            return Some(child.utf8_text(source).ok()?.to_string());
        }
    }
    None
}

/// Extract a name string from a node, handling nested patterns.
fn name_from_node(node: &tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let kind = node.kind();
    // Direct identifier nodes
    if kind == "identifier"
        || kind == "type_identifier"
        || kind == "property_identifier"
        || kind == "field_identifier"
    {
        return Some(node.utf8_text(source).ok()?.to_string());
    }
    // Pointer declarators, reference declarators, etc. (C/C++)
    if kind.contains("declarator") {
        if let Some(inner) = node.child_by_field_name("declarator") {
            return name_from_node(&inner, source);
        }
        // Or a direct identifier child
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "field_identifier" {
                return Some(child.utf8_text(source).ok()?.to_string());
            }
        }
    }
    None
}

/// Extract top-level symbols from a parsed tree.
///
/// Walks the AST, identifying nodes that match the language's extraction rules.
/// Returns symbols sorted by their position in the source.
pub fn extract_symbols(tree: &tree_sitter::Tree, source: &[u8], lang: Language) -> Vec<RawSymbol> {
    let mut symbols = Vec::new();
    collect_symbols(tree.root_node(), source, lang, &mut symbols, 0);
    symbols.sort_by_key(|s| s.start_byte);
    symbols
}

/// Recursively collect symbols from AST nodes.
///
/// `depth` limits how deep we recurse to avoid extracting deeply nested items
/// as top-level symbols. We go up to depth 3 to handle patterns like:
/// - export_statement > function_declaration (TS/JS)
/// - decorated_definition > function_definition (Python)
/// - impl_item > function_item (Rust methods)
fn collect_symbols(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    lang: Language,
    symbols: &mut Vec<RawSymbol>,
    depth: usize,
) {
    if depth > 4 {
        return;
    }

    let kind = node.kind();

    // Handle Go type_declaration → recurse into type_spec children
    if lang == Language::Go && kind == "type_declaration" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                let sym_type = refine_go_type_spec(&child, source);
                let name = extract_name(&child, source);
                symbols.push(RawSymbol {
                    name,
                    symbol_type: sym_type,
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                    start_row: child.start_position().row,
                    end_row: child.end_position().row,
                });
            }
        }
        return;
    }

    if let Some(sym_type) = classify_node(lang, kind) {
        // For Rust impl blocks, extract methods inside but also keep the whole block
        if lang == Language::Rust && kind == "impl_item" {
            extract_impl_methods(node, source, lang, symbols);
            return;
        }

        let name = extract_name(&node, source);
        symbols.push(RawSymbol {
            name,
            symbol_type: sym_type,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_row: node.start_position().row,
            end_row: node.end_position().row,
        });
        return;
    }

    // Recurse into children for wrapper nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_symbols(child, source, lang, symbols, depth + 1);
    }
}

/// Refine a Go type_spec into the correct SymbolType based on the type child.
fn refine_go_type_spec(node: &tree_sitter::Node<'_>, source: &[u8]) -> SymbolType {
    if let Some(type_child) = node.child_by_field_name("type") {
        match type_child.kind() {
            "struct_type" => return SymbolType::Struct,
            "interface_type" => return SymbolType::Interface,
            _ => {}
        }
    }
    // Check the text for common patterns
    let text = node.utf8_text(source).unwrap_or("");
    if text.contains("struct") {
        SymbolType::Struct
    } else if text.contains("interface") {
        SymbolType::Interface
    } else {
        SymbolType::TypeAlias
    }
}

/// Extract individual methods from a Rust `impl` block as separate symbols.
fn extract_impl_methods(
    impl_node: tree_sitter::Node<'_>,
    source: &[u8],
    lang: Language,
    symbols: &mut Vec<RawSymbol>,
) {
    let mut cursor = impl_node.walk();
    let mut found_methods = false;

    for child in impl_node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            let mut inner_cursor = child.walk();
            for item in child.children(&mut inner_cursor) {
                if item.kind() == "function_item" {
                    let name = extract_name(&item, source);
                    symbols.push(RawSymbol {
                        name,
                        symbol_type: SymbolType::Method,
                        start_byte: item.start_byte(),
                        end_byte: item.end_byte(),
                        start_row: item.start_position().row,
                        end_row: item.end_position().row,
                    });
                    found_methods = true;
                } else if let Some(sym_type) = classify_node(lang, item.kind()) {
                    let name = extract_name(&item, source);
                    symbols.push(RawSymbol {
                        name,
                        symbol_type: sym_type,
                        start_byte: item.start_byte(),
                        end_byte: item.end_byte(),
                        start_row: item.start_position().row,
                        end_row: item.end_position().row,
                    });
                    found_methods = true;
                }
            }
        }
    }

    // If impl has no methods, capture the whole block
    if !found_methods {
        let name = extract_name(&impl_node, source);
        symbols.push(RawSymbol {
            name,
            symbol_type: SymbolType::Block,
            start_byte: impl_node.start_byte(),
            end_byte: impl_node.end_byte(),
            start_row: impl_node.start_position().row,
            end_row: impl_node.end_position().row,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::languages::tree_sitter_grammar;

    fn parse_and_extract(source: &str, lang: Language) -> Vec<RawSymbol> {
        let grammar = tree_sitter_grammar(lang).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar).unwrap();
        let tree = parser.parse(source, None).unwrap();
        extract_symbols(&tree, source.as_bytes(), lang)
    }

    #[test]
    fn rust_extracts_functions() {
        let source = r#"
fn hello() {
    println!("hello");
}

fn world(x: i32) -> i32 {
    x + 1
}
"#;
        let symbols = parse_and_extract(source, Language::Rust);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name.as_deref(), Some("hello"));
        assert_eq!(symbols[0].symbol_type, SymbolType::Function);
        assert_eq!(symbols[1].name.as_deref(), Some("world"));
    }

    #[test]
    fn rust_extracts_structs_and_enums() {
        let source = r#"
struct Point {
    x: f64,
    y: f64,
}

enum Color {
    Red,
    Green,
    Blue,
}
"#;
        let symbols = parse_and_extract(source, Language::Rust);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].symbol_type, SymbolType::Struct);
        assert_eq!(symbols[0].name.as_deref(), Some("Point"));
        assert_eq!(symbols[1].symbol_type, SymbolType::Enum);
        assert_eq!(symbols[1].name.as_deref(), Some("Color"));
    }

    #[test]
    fn rust_extracts_impl_methods() {
        let source = r#"
impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn distance(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
"#;
        let symbols = parse_and_extract(source, Language::Rust);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].symbol_type, SymbolType::Method);
        assert_eq!(symbols[0].name.as_deref(), Some("new"));
        assert_eq!(symbols[1].symbol_type, SymbolType::Method);
        assert_eq!(symbols[1].name.as_deref(), Some("distance"));
    }

    #[test]
    fn rust_extracts_traits() {
        let source = r#"
trait Drawable {
    fn draw(&self);
    fn area(&self) -> f64;
}
"#;
        let symbols = parse_and_extract(source, Language::Rust);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol_type, SymbolType::Trait);
        assert_eq!(symbols[0].name.as_deref(), Some("Drawable"));
    }

    #[test]
    fn python_extracts_functions_and_classes() {
        let source = r#"
def hello():
    print("hello")

class MyClass:
    def __init__(self):
        self.x = 0

    def method(self):
        return self.x
"#;
        let symbols = parse_and_extract(source, Language::Python);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].symbol_type, SymbolType::Function);
        assert_eq!(symbols[0].name.as_deref(), Some("hello"));
        assert_eq!(symbols[1].symbol_type, SymbolType::Class);
        assert_eq!(symbols[1].name.as_deref(), Some("MyClass"));
    }

    #[test]
    fn typescript_extracts_functions_and_interfaces() {
        let source = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}

interface User {
    name: string;
    age: number;
}

class UserService {
    private users: User[];

    getUser(id: number): User {
        return this.users[id];
    }
}
"#;
        let symbols = parse_and_extract(source, Language::TypeScript);
        assert!(
            symbols.len() >= 3,
            "expected >= 3 symbols, got {}",
            symbols.len()
        );

        let func = symbols.iter().find(|s| s.name.as_deref() == Some("greet"));
        assert!(func.is_some(), "should find function 'greet'");
        assert_eq!(func.unwrap().symbol_type, SymbolType::Function);

        let iface = symbols.iter().find(|s| s.name.as_deref() == Some("User"));
        assert!(iface.is_some(), "should find interface 'User'");
        assert_eq!(iface.unwrap().symbol_type, SymbolType::Interface);

        let class = symbols
            .iter()
            .find(|s| s.name.as_deref() == Some("UserService"));
        assert!(class.is_some(), "should find class 'UserService'");
        assert_eq!(class.unwrap().symbol_type, SymbolType::Class);
    }

    #[test]
    fn go_extracts_functions_and_structs() {
        let source = r#"
package main

func Hello() string {
    return "hello"
}

type Point struct {
    X float64
    Y float64
}

func (p *Point) Distance() float64 {
    return p.X * p.X + p.Y * p.Y
}
"#;
        let symbols = parse_and_extract(source, Language::Go);
        assert!(
            symbols.len() >= 3,
            "expected >= 3 symbols, got {}",
            symbols.len()
        );

        let func = symbols.iter().find(|s| s.name.as_deref() == Some("Hello"));
        assert!(func.is_some(), "should find function 'Hello'");
        assert_eq!(func.unwrap().symbol_type, SymbolType::Function);

        let struc = symbols.iter().find(|s| s.name.as_deref() == Some("Point"));
        assert!(struc.is_some(), "should find struct 'Point'");
        assert_eq!(struc.unwrap().symbol_type, SymbolType::Struct);

        let method = symbols
            .iter()
            .find(|s| s.name.as_deref() == Some("Distance"));
        assert!(method.is_some(), "should find method 'Distance'");
        assert_eq!(method.unwrap().symbol_type, SymbolType::Method);
    }

    #[test]
    fn java_extracts_class_and_methods() {
        let source = r#"
class Calculator {
    public int add(int a, int b) {
        return a + b;
    }

    public int multiply(int a, int b) {
        return a * b;
    }
}
"#;
        let symbols = parse_and_extract(source, Language::Java);
        assert!(
            symbols.len() >= 1,
            "expected >= 1 symbol, got {}",
            symbols.len()
        );

        let class = symbols
            .iter()
            .find(|s| s.name.as_deref() == Some("Calculator"));
        assert!(class.is_some(), "should find class 'Calculator'");
        assert_eq!(class.unwrap().symbol_type, SymbolType::Class);
    }

    #[test]
    fn c_extracts_functions_and_structs() {
        let source = r#"
struct Point {
    double x;
    double y;
};

double distance(struct Point* p) {
    return p->x * p->x + p->y * p->y;
}
"#;
        let symbols = parse_and_extract(source, Language::C);
        assert!(
            symbols.len() >= 1,
            "expected >= 1 symbol, got {}",
            symbols.len()
        );

        let func = symbols
            .iter()
            .find(|s| s.symbol_type == SymbolType::Function);
        assert!(func.is_some(), "should find a function");
        assert_eq!(func.unwrap().name.as_deref(), Some("distance"));
    }

    #[test]
    fn cpp_extracts_classes_and_functions() {
        let source = r#"
class Shape {
public:
    virtual double area() = 0;
};

namespace geometry {
    double pi() {
        return 3.14159;
    }
}
"#;
        let symbols = parse_and_extract(source, Language::Cpp);
        assert!(
            symbols.len() >= 1,
            "expected >= 1 symbol, got {}",
            symbols.len()
        );

        let class = symbols.iter().find(|s| s.symbol_type == SymbolType::Class);
        assert!(class.is_some(), "should find a class");

        let ns = symbols.iter().find(|s| s.symbol_type == SymbolType::Module);
        assert!(ns.is_some(), "should find a namespace");
    }
}
