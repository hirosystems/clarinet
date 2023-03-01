use clarity_repl::clarity::SymbolicExpression;
use regex::Regex;

/*
A comment can be:
- before an expression, on the line above
- after an expression, at the end of the line

    ;; comment before `ok`
    (ok (+ 1 2)) ;; comment after `ok`
    ;; comment before `print`
    (print "hello")
*/

// detached comments only contain the line number and the content
#[derive(Debug, PartialEq)]
pub enum DetachedComment {
    Before(u32, String),
    After(u32, String),
}

// attached comments are attached to an symbolic expression id
#[derive(Debug, PartialEq)]
pub enum AttachedComment {
    Before(u64, String),
    After(u64, String),
}

pub fn extract_comments(source: &str) -> Vec<DetachedComment> {
    let mut comments = vec![];
    let before_re = Regex::new(r"^[ \t]*?;;(.*)$").unwrap();
    let after_re = Regex::new(r"^.*?\S.*?;;(.*)$").unwrap();

    for (i, line) in source.lines().enumerate() {
        if let Some(comment) = before_re.captures(line) {
            comments.push(DetachedComment::Before(
                i as u32,
                comment[1].trim().to_owned(),
            ));
            continue;
        }

        if let Some(comment) = after_re.captures(line) {
            comments.push(DetachedComment::After(
                i as u32,
                comment[1].trim().to_owned(),
            ));
        }
    }
    comments
}

#[cfg(test)]
mod tests_extract_comments {
    use super::extract_comments;
    use super::DetachedComment;

    #[test]
    fn match_comments_before_expr() {
        let src = vec![
            ";; no whitespace",
            "(print true)",
            "  ;; with spaces",
            "(print true)",
            "\t;; with 1 tab",
            "(print true)",
            ";;no-space",
            ";;still;;valid ;; comment",
        ]
        .join("\n");

        let comments = extract_comments(&String::from(src));
        assert_eq!(comments.len(), 5);
        assert_eq!(
            comments[0],
            DetachedComment::Before(0, "no whitespace".to_string())
        );
        assert_eq!(
            comments[1],
            DetachedComment::Before(2, "with spaces".to_string())
        );
        assert_eq!(
            comments[2],
            DetachedComment::Before(4, "with 1 tab".to_string())
        );
        assert_eq!(
            comments[3],
            DetachedComment::Before(6, "no-space".to_string())
        );
        assert_eq!(
            comments[4],
            DetachedComment::Before(7, "still;;valid ;; comment".to_string())
        );
    }

    #[test]
    fn match_comments_after_expr() {
        let src = vec![
            "(print true) ;; print stuff",
            "(print true);;valid comment",
            "(print true);;still;;valid ;; comment",
        ]
        .join("\n");

        let comments = extract_comments(&String::from(src));
        assert_eq!(comments.len(), 3);
        assert_eq!(
            comments[0],
            DetachedComment::After(0, "print stuff".to_string())
        );
        assert_eq!(
            comments[1],
            DetachedComment::After(1, "valid comment".to_string())
        );
        assert_eq!(
            comments[2],
            DetachedComment::After(2, "still;;valid ;; comment".to_string())
        );
    }

    #[test]
    fn match_comments_expr() {
        let src = ";; comment\n(print true) ;; print stuff";

        let comments = extract_comments(&String::from(src));
        assert_eq!(comments.len(), 2);
        assert_eq!(
            comments[0],
            DetachedComment::Before(0, "comment".to_string())
        );
        assert_eq!(
            comments[1],
            DetachedComment::After(1, "print stuff".to_string())
        );
    }
}

fn get_closest_expr_id_start_after(expressions: &[SymbolicExpression], line: u32) -> Option<u64> {
    for expr in expressions {
        if line < expr.span.start_line {
            return Some(expr.id);
        }
        if line < expr.span.end_line {
            if let Some(list) = expr.match_list() {
                return get_closest_expr_id_start_after(list, line);
            }
        }
    }
    None
}

fn get_closest_expr_id_ending_before(expressions: &[SymbolicExpression], line: u32) -> Option<u64> {
    for expr in expressions {
        if line == expr.span.end_line {
            return Some(expr.id);
        }
        if line < expr.span.end_line {
            if let Some(list) = expr.match_list() {
                return get_closest_expr_id_ending_before(list, line);
            }
        }
    }
    None
}

pub fn attach_comments_to_ast(
    comments: &Vec<DetachedComment>,
    expressions: &Vec<SymbolicExpression>,
) -> Vec<AttachedComment> {
    let mut result = vec![];
    // @todo: empty comments vec as we iterate?
    for comment in comments {
        match comment {
            DetachedComment::Before(line, comment) => {
                if let Some(expr_id) = get_closest_expr_id_start_after(expressions, line + 1) {
                    result.push(AttachedComment::Before(expr_id, comment.to_string()))
                } // @todo: handle `else` (edge case)
            }
            DetachedComment::After(line, comment) => {
                if let Some(expr_id) = get_closest_expr_id_ending_before(expressions, line + 1) {
                    result.push(AttachedComment::After(expr_id, comment.to_string()))
                } // @todo: handle `else` (edge case)
            }
        };
    }

    result
}
#[cfg(test)]
mod tests_attach_comments {
    use clarity_repl::clarity::ast::{build_ast_with_rules, ContractAST};
    use clarity_repl::clarity::stacks_common::types::StacksEpochId;
    use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
    use clarity_repl::clarity::ClarityVersion;

    use crate::formatter::comments::AttachedComment;

    use super::{attach_comments_to_ast, extract_comments};

    fn get_ast(source: &str) -> ContractAST {
        build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap()
    }

    #[test]
    fn attach_basic_before_comment() {
        let src = vec![";; comment", "(print true)"].join("\n");

        let comments = extract_comments(&src);
        let ast = get_ast(&src);
        let result = attach_comments_to_ast(&comments, &ast.expressions);
        assert_eq!(
            result,
            vec![AttachedComment::Before(1, "comment".to_string())]
        );
    }

    #[test]
    fn attach_before_comments() {
        let src = vec![
            ";; one",
            "u1",
            ";; comment",
            "(define-public (add)",
            "  ;; ok2",
            "  (ok (+ 1 1))",
            ")",
            ";; comment",
            "(define-public (sub)",
            "  (ok",
            "    ;; ok0",
            "    (- 1 1)",
            "  )",
            ")",
        ]
        .join("\n");

        let comments = extract_comments(&src);
        let ast = get_ast(&src);
        let r = attach_comments_to_ast(&comments, &ast.expressions);
        assert_eq!(r.len(), 5);
        assert_eq!(r[0], AttachedComment::Before(1, "one".to_string()));
        assert_eq!(r[1], AttachedComment::Before(2, "comment".to_string()));
        assert_eq!(r[2], AttachedComment::Before(6, "ok2".to_string()));
        assert_eq!(r[3], AttachedComment::Before(12, "comment".to_string()));
        assert_eq!(r[4], AttachedComment::Before(18, "ok0".to_string()));
    }

    #[test]
    fn attach_basic_after_comment() {
        let src = "(print true) ;; comment";

        let comments = extract_comments(&src);
        let ast = get_ast(&src);
        let result = attach_comments_to_ast(&comments, &ast.expressions);
        assert_eq!(
            result,
            vec![AttachedComment::After(1, "comment".to_string())]
        );
    }

    #[test]
    fn attach_after_comments() {
        let src = vec![
            "u1 ;; one",
            "(define-public (add) ;; add",
            "  (ok (+ 1 1)) ;; ok2",
            ") ;; end add",
            "(define-public (sub) ;; sub",
            "  (ok",
            "    (- 1 1) ;; ok0",
            "  ) ;; end ok",
            ")",
        ]
        .join("\n");

        let comments = extract_comments(&src);
        let ast = get_ast(&src);
        let r = attach_comments_to_ast(&comments, &ast.expressions);
        assert_eq!(r.len(), 7);
        assert_eq!(r[0], AttachedComment::After(1, "one".to_string()));
        assert_eq!(r[1], AttachedComment::After(3, "add".to_string()));
        assert_eq!(r[2], AttachedComment::After(6, "ok2".to_string()));
        assert_eq!(r[3], AttachedComment::After(2, "end add".to_string()));
        assert_eq!(r[4], AttachedComment::After(13, "sub".to_string()));
        assert_eq!(r[5], AttachedComment::After(18, "ok0".to_string()));
        assert_eq!(r[6], AttachedComment::After(16, "end ok".to_string()));
    }

    #[test]
    fn test_attach_comments() {
        let src = vec![
            ";; add",
            "(define-public (add) ;; begin",
            "  ;; return",
            "  (ok (+ 1 1)) ;; 2",
            ") ;; end",
        ]
        .join("\n");

        let comments = extract_comments(&src);
        let ast = get_ast(&src);
        let r = attach_comments_to_ast(&comments, &ast.expressions);
        assert_eq!(r.len(), 5);
        assert_eq!(r[0], AttachedComment::Before(1, "add".to_string()));
        assert_eq!(r[1], AttachedComment::After(2, "begin".to_string()));
        assert_eq!(r[2], AttachedComment::Before(5, "return".to_string()));
        assert_eq!(r[3], AttachedComment::After(5, "2".to_string()));
        assert_eq!(r[4], AttachedComment::After(1, "end".to_string()));
    }
}
