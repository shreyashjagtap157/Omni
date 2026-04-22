use std::env;
use std::fs::File;
use std::io::{Result, Write};
use std::path::PathBuf;

fn emit_test(file: &mut File, name: &str, source: &str) -> Result<()> {
    writeln!(file, "#[test]")?;
    writeln!(file, "fn {name}() {{")?;
    writeln!(file, "    assert_roundtrip_ok({source:?});")?;
    writeln!(file, "}}
")?;
    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest = out_dir.join("generated_regressions.rs");
    let mut file = File::create(dest)?;

    writeln!(file, "use omni_compiler::formatter;")?;
    writeln!(file, "use omni_compiler::lexer::Lexer;")?;
    writeln!(file, "use omni_compiler::parser::Parser;")?;
    writeln!(file, "use omni_compiler::resolver;")?;
    writeln!(file, "use omni_compiler::type_checker;")?;
    writeln!(file)?;
    writeln!(file, "fn assert_roundtrip_ok(src: &str) {{")?;
    writeln!(file, "    let mut lexer = Lexer::new(src);")?;
    writeln!(file, "    let tokens = lexer.tokenize().expect(\"tokenize failed\");")?;
    writeln!(file, "    let mut parser = Parser::new(tokens);")?;
    writeln!(file, "    let program = parser.parse_program().expect(\"parse failed\");")?;
    writeln!(file, "    resolver::resolve_program(&program).expect(\"resolve failed\");")?;
    writeln!(file, "    type_checker::type_check_program(&program).expect(\"typecheck failed\");")?;
    writeln!(file, "    let formatted = formatter::format_program(&program);")?;
    writeln!(file, "    assert!(!formatted.is_empty());")?;
    writeln!(file, "    let mut lexer2 = Lexer::new(&formatted);")?;
    writeln!(file, "    let tokens2 = lexer2.tokenize().expect(\"re-tokenize failed\");")?;
    writeln!(file, "    let mut parser2 = Parser::new(tokens2);")?;
    writeln!(file, "    let program2 = parser2.parse_program().expect(\"reparse failed\");")?;
    writeln!(file, "    resolver::resolve_program(&program2).expect(\"resolve2 failed\");")?;
    writeln!(file, "    type_checker::type_check_program(&program2).expect(\"typecheck2 failed\");")?;
    writeln!(file, "    let reformatted = formatter::format_program(&program2);")?;
    writeln!(file, "    assert!(!reformatted.is_empty());")?;
    writeln!(file, "}}
")?;

    for i in 0..50 {
        let a = i as i64;
        let b = a + 1;

        let arithmetic = format!(
            "let value{idx} = {a} + {b}\nprint value{idx}\n",
            idx = i,
            a = a,
            b = b
        );
        emit_test(&mut file, &format!("arith_roundtrip_{i:03}"), &arithmetic)?;

        let function = format!(
            "fn id{idx}(x)\n    return x\nlet value{idx} = id{idx}({a})\nprint value{idx}\n",
            idx = i,
            a = a
        );
        emit_test(&mut file, &format!("function_roundtrip_{i:03}"), &function)?;

        let tuple_range = format!(
            "let tuple{idx} = ({a}, {b})\nlet range{idx} = {a}..{c}\nprint {a}\n",
            idx = i,
            a = a,
            b = b,
            c = a + 3
        );
        emit_test(&mut file, &format!("tuple_range_roundtrip_{i:03}"), &tuple_range)?;

        let literals = format!(
            "let flag{idx} = true\nlet text{idx} = \"s{idx}\"\nprint text{idx}\n",
            idx = i
        );
        emit_test(&mut file, &format!("literal_roundtrip_{i:03}"), &literals)?;
    }

    Ok(())
}
