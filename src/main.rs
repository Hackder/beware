//! The simplest linter

use std::{env, path::Path};

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_diagnostics::{
    miette::{self, Diagnostic},
    thiserror::Error,
};
use oxc_parser::Parser;
use oxc_semantic::{SemanticBuilder, SymbolId};
use oxc_span::{GetSpan, SourceType, Span};

// Instruction:
// create a `test.js`,
// run `cargo run -p oxc_linter --example linter`
// or `cargo watch -x "run -p oxc_linter --example linter"`

fn main() {
    let name = env::args().nth(1).unwrap_or_else(|| "test.ts".to_string());
    let path = Path::new(&name);
    let source_text = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("{name} not found"));
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap();
    let ret = Parser::new(&allocator, &source_text, source_type).parse();

    // Handle parser errors
    if !ret.errors.is_empty() {
        print_errors(&source_text, ret.errors);
        return;
    }

    let program = allocator.alloc(ret.program);
    let semantic_ret = SemanticBuilder::new(&source_text, source_type)
        .with_trivias(ret.trivias)
        .build(program);

    let mut errors: Vec<oxc_diagnostics::Error> = vec![];

    // for symbol_id in semantic_ret.semantic.symbols().iter() {
    //     let decl = semantic_ret.semantic.symbol_declaration(symbol_id);
    //
    //     match decl.kind() {
    //         AstKind::Function(func) => {
    //             println!("{:?}", func);
    //         }
    //         _ => {}
    //     }
    // }

    for node in semantic_ret.semantic.nodes().iter() {
        match node.kind() {
            AstKind::DebuggerStatement(stmt) => {
                errors.push(NoDebugger(stmt.span).into());
            }
            AstKind::ArrayPattern(array) if array.elements.is_empty() => {
                errors.push(NoEmptyPattern("array", array.span).into());
            }
            AstKind::ObjectPattern(object) if object.properties.is_empty() => {
                errors.push(NoEmptyPattern("object", object.span).into());
            }
            AstKind::CallExpression(call) => {
                let reference_id = call.callee.get_identifier_reference().unwrap().reference_id.get().unwrap();
                let reference = semantic_ret.semantic.symbols().get_reference(reference_id);

                let decl = semantic_ret.semantic.symbol_declaration(reference.symbol_id().unwrap());
                println!("{:?}", decl);
                // println!("{:?} --------------------------", refernce_id);
                errors.push(NoDebugger(decl.kind().span()).into());
            }
            _ => {}
        }
    }

    if !errors.is_empty() {
        print_errors(&source_text, errors);
        return;
    }

    println!("Success!");
}

fn print_errors(source_text: &str, errors: Vec<oxc_diagnostics::Error>) {
    for error in errors {
        let error = error.with_source_code(source_text.to_string());
        println!("{error:?}");
    }
}

// This prints:
//
//   ⚠ `debugger` statement is not allowed
//   ╭────
// 1 │ debugger;
//   · ─────────
//   ╰────
#[derive(Debug, Error, Diagnostic)]
#[error("`debugger` statement is not allowed")]
#[diagnostic(severity(warning))]
struct NoDebugger(#[label] pub Span);

// This prints:
//
//   ⚠ empty destructuring pattern is not allowed
//   ╭────
// 1 │ let {} = {};
//   ·     ─┬
//   ·      ╰── Empty object binding pattern
//   ╰────
#[derive(Debug, Error, Diagnostic)]
#[error("empty destructuring pattern is not allowed")]
#[diagnostic(severity(warning))]
struct NoEmptyPattern(&'static str, #[label("Empty {0} binding pattern")] pub Span);

