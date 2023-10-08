use crate::types::*;

pub struct Parser;

impl Parser {
    pub fn parse(raw: &str) -> PatternSection {
        let mut stack: Vec<PatternSection> = vec![];
        let mut ops: Vec<Op> = vec![];

        let mut need_and = false;
        let mut idx = 0usize;

        let mut raw_it = raw.chars();
        while let Some(c) = raw_it.next() {
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
            } else if c == '[' {
                let mut next_c = raw_it.next().expect("Missing end of char group");
                let is_negated = next_c == '^';
                let mut char_group_chars = vec![];

                if next_c == '^' {
                    next_c = raw_it.next().expect("Missing end of char group");
                }

                loop {
                    if next_c == ']' {
                        break;
                    }

                    char_group_chars.push(next_c);

                    next_c = raw_it.next().expect("Missing end of char group");
                }

                stack.push(PatternSection::CharGroup(
                    char_group_chars,
                    Mod::One,
                    is_negated,
                ));

                if need_and {
                    ops.push(Op::And);
                }
                need_and = true;
            } else if c == '{' {
                let mut min_str = String::new();
                let mut min_is_max = false;
                let min: usize;
                let max: usize;

                loop {
                    let next_c = raw_it.next().expect("Missing char");
                    if next_c == ',' {
                        break;
                    } else if next_c == '}' {
                        min_is_max = true;
                        break;
                    }
                    min_str.push(next_c);
                }

                min = usize::from_str_radix(&min_str, 10).expect("Invalid number");
                if !min_is_max {
                    let mut max_str = String::new();
                    loop {
                        let next_c = raw_it.next().expect("Missing char");
                        if next_c == '}' {
                            break;
                        }
                        max_str.push(next_c);
                    }

                    max = usize::from_str_radix(&max_str, 10).expect("Invalid number");
                } else {
                    max = min;
                }

                Parser::inject_mod(&mut stack, Mod::Range(min, max));
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
            PatternSection::CharGroup(v, _, is_negated) => {
                PatternSection::CharGroup(v, m, is_negated)
            }
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
    fn test_char_group() {
        assert_eq!(
            PatternSection::And(
                vec![
                    PatternSection::Char('a', Mod::One),
                    PatternSection::CharGroup(vec!['b', 'c'], Mod::One, false),
                    PatternSection::Char('d', Mod::One),
                ],
                Mod::One
            ),
            Parser::parse("a[bc]d"),
        );
        assert_eq!(
            PatternSection::Or(
                vec![
                    PatternSection::Char('a', Mod::One),
                    PatternSection::CharGroup(vec!['b', 'c'], Mod::One, true),
                ],
                Mod::One
            ),
            Parser::parse("a|[^bc]"),
        );
        assert_eq!(
            PatternSection::And(
                vec![
                    PatternSection::CharGroup(vec!['b', 'c'], Mod::Any, true),
                    PatternSection::Char('a', Mod::One),
                ],
                Mod::One
            ),
            Parser::parse("[^bc]*a"),
        );
    }

    #[test]
    fn test_mod_range() {
        assert_eq!(
            PatternSection::Char('a', Mod::Range(3, 3)),
            Parser::parse("a{3}"),
        );

        assert_eq!(
            PatternSection::Char('a', Mod::Range(3, 6)),
            Parser::parse("a{3,6}"),
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
