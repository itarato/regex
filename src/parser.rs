use crate::types::*;

pub struct Parser;

impl Parser {
    pub fn parse(raw: &str) -> PatternSection {
        let mut stack: Vec<PatternSection> = vec![];
        let mut ops: Vec<Op> = vec![];

        let mut need_and = false;
        let mut idx = 0usize;

        for c in raw.chars() {
            if let Some(pattern_mod) = Mod::from(&c) {
                Parser::inject_mod(&mut stack, pattern_mod);
            } else if c == '|' {
                Parser::collapse_stacks(&mut stack, &mut ops, |op| match op {
                    Some(Op::And) => false,
                    _ => true,
                });
                ops.push(Op::Or);
                need_and = false;
            } else if c == '(' {
                if need_and {
                    ops.push(Op::And)
                }
                need_and = false;
                ops.push(Op::Paren)
            } else if c == ')' {
                Parser::collapse_stacks(&mut stack, &mut ops, |op| match op {
                    Some(Op::Paren) => true,
                    _ => false,
                });
                assert_eq!(Some(Op::Paren), ops.pop());
                if idx < raw.len() - 1 {
                    need_and = true;
                }
            } else if c.is_ascii_alphanumeric() || c == '.' {
                stack.push(PatternSection::Char(c, Mod::One));
                if need_and {
                    ops.push(Op::And);
                }
                need_and = true;
            } else {
                panic!("Unexpected character error");
            }

            idx += 1;
        }

        Parser::collapse_stacks(&mut stack, &mut ops, |op| match op {
            None => true,
            Some(_) => false,
        });
        assert!(ops.is_empty());
        assert!(stack.len() <= 1);

        stack.pop().unwrap_or(PatternSection::And(vec![], Mod::One))
    }

    fn collapse_stacks(
        stack: &mut Vec<PatternSection>,
        ops: &mut Vec<Op>,
        until: fn(Option<&Op>) -> bool,
    ) {
        loop {
            if until(ops.last()) {
                return;
            }

            let (op, count) = Parser::pop_same(ops);
            if op.is_none() {
                return;
            }
            let op = op.unwrap();
            let tail = stack.drain(stack.len() - count - 1..).collect::<Vec<_>>();

            let collapsed = match op {
                Op::And => PatternSection::And(tail, Mod::One),
                Op::Or => PatternSection::Or(tail, Mod::One),
                _ => unreachable!("Unexpected OP during collapse"),
            };
            stack.push(collapsed);
        }
    }

    fn pop_same(ops: &mut Vec<Op>) -> (Option<Op>, usize) {
        let top_op = ops.last();
        if top_op.is_none() {
            return (None, 0);
        }
        let top_op = *top_op.unwrap();

        let mut count = 0;
        loop {
            if Some(&top_op) != ops.last() {
                break;
            }
            ops.pop();
            count += 1;
        }

        (Some(top_op), count)
    }

    fn inject_mod(stack: &mut Vec<PatternSection>, m: Mod) {
        let new_pattern = match stack.pop().expect("Empty stack error") {
            PatternSection::And(v, _) => PatternSection::And(v, m),
            PatternSection::Or(v, _) => PatternSection::Or(v, m),
            PatternSection::Char(v, _) => PatternSection::Char(v, m),
        };

        stack.push(new_pattern);
    }
}

#[cfg(test)]
mod test {
    use crate::parser::*;

    #[test]
    fn test_empty() {
        assert_eq!(PatternSection::And(vec![], Mod::One), Parser::parse(""));
    }

    #[test]
    fn test_and() {
        assert_eq!(
            PatternSection::And(
                vec![
                    PatternSection::Char('a', Mod::One),
                    PatternSection::Char('b', Mod::OneOrMore),
                    PatternSection::Char('c', Mod::ZeroOrOne),
                    PatternSection::Char('d', Mod::Any),
                ],
                Mod::One
            ),
            Parser::parse("ab+c?d*")
        );
    }

    #[test]
    fn test_or() {
        assert_eq!(
            PatternSection::Or(
                vec![
                    PatternSection::Char('a', Mod::One),
                    PatternSection::Char('b', Mod::Any),
                ],
                Mod::One
            ),
            Parser::parse("a|b*")
        );
    }

    #[test]
    fn test_mixed() {
        assert_eq!(
            PatternSection::Or(
                vec![
                    PatternSection::And(
                        vec![
                            PatternSection::Char('a', Mod::One),
                            PatternSection::Char('b', Mod::ZeroOrOne),
                        ],
                        Mod::One
                    ),
                    PatternSection::Or(
                        vec![
                            PatternSection::And(
                                vec![
                                    PatternSection::Char('c', Mod::One),
                                    PatternSection::Char('d', Mod::One),
                                ],
                                Mod::One
                            ),
                            PatternSection::Or(
                                vec![
                                    PatternSection::And(
                                        vec![
                                            PatternSection::Char('1', Mod::One),
                                            PatternSection::Char('f', Mod::One),
                                        ],
                                        Mod::One,
                                    ),
                                    PatternSection::And(
                                        vec![
                                            PatternSection::Char('g', Mod::One),
                                            PatternSection::Char('h', Mod::One),
                                        ],
                                        Mod::One,
                                    ),
                                    PatternSection::And(
                                        vec![
                                            PatternSection::Char('i', Mod::One),
                                            PatternSection::Char('j', Mod::One),
                                        ],
                                        Mod::One,
                                    ),
                                ],
                                Mod::ZeroOrOne,
                            ),
                        ],
                        Mod::Any,
                    ),
                ],
                Mod::One,
            ),
            Parser::parse("ab?|(cd|(1f|gh|ij)?)*"),
        );
    }
}
