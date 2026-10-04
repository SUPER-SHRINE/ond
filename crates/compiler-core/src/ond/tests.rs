#[cfg(test)]
mod tests {
    use super::{parse_source_file, FileId};

    #[test]
    fn parses_explicit_import_aliases() {
        let file = parse_source_file(
            FileId(0),
            "package main\nimport std_json \"json\"\nimport \"my/json\"\nfunc main() {}\n",
        )
        .unwrap();
        assert_eq!(file.imports[0].alias.as_ref().unwrap().name, "std_json");
        assert!(file.imports[1].alias.is_none());
    }
    use crate::ir::ast;

    #[test]
    fn parses_function_with_multi_return_signature() {
        let file = parse_source_file(
            FileId(0),
            "package main\n\nfunc pair(value: u32) -> (u32, u32) {\n    return value, value\n}\n",
        )
        .expect("source should parse");

        assert_eq!(file.package.name, "main");
        assert_eq!(file.decls.len(), 1);
        match &file.decls[0] {
            ast::TopLevelDecl::Func(func) => {
                assert_eq!(func.name.name, "pair");
                assert_eq!(func.signature.params.len(), 1);
                assert_eq!(func.signature.results.len(), 2);
            }
            _ => panic!("expected function declaration"),
        }
    }

    #[test]
    fn inserts_semicolons_on_newlines_in_block() {
        parse_source_file(
            FileId(0),
            "package main\n\nfunc main() {\n    value := 1\n    if value == 1 {\n        value = value + 1\n    }\n}\n",
        )
        .expect("newlines should act as statement separators");
    }

    #[test]
    fn ignores_line_and_block_comments_during_parse() {
        let file = parse_source_file(
            FileId(0),
            "package main\n\
            // package-level line comment\n\
            \n\
            /* block comment before function */\n\
            func main() {\n\
                value := 1 // trailing line comment\n\
                /* multi-line\n\
                   block comment */\n\
                value = value + 1\n\
            }\n",
        )
        .expect("comments should be ignored like whitespace");

        assert_eq!(file.package.name, "main");
        assert_eq!(file.decls.len(), 1);
    }

    #[test]
    fn preserves_byte_spans_after_unicode_comments() {
        let file = parse_source_file(
            FileId(0),
            "package main\n\
            // 日本語の行コメント\n\
            /* 日本語のブロックコメント */\n\
            func main() {\n\
                value := 1 /* 値の説明 */\n\
                value = value + 1\n\
            }\n",
        )
        .expect("unicode comments must not corrupt token spans");

        assert_eq!(file.package.name, "main");
        assert_eq!(file.decls.len(), 1);
    }

    #[test]
    fn treats_multiline_block_comment_as_newline_for_semicolon_insertion() {
        parse_source_file(
            FileId(0),
            "package main\n\nfunc main() {\n    value := 1 /* comment\nwith newline */\n    value = value + 1\n}\n",
        )
        .expect("multiline block comment should preserve semicolon insertion");
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let error = parse_source_file(FileId(0), "package main\n\n/* missing end\n")
            .expect_err("unterminated block comment must fail");
        let rendered = error
            .into_vec()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("unterminated block comment"));
    }

    #[test]
    fn reports_missing_package_clause() {
        let error = parse_source_file(FileId(0), "func main() {}\n")
            .expect_err("missing package clause must fail");
        let rendered = error
            .into_vec()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("package"));
    }
}
