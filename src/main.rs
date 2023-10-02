use std::{collections::HashMap, env::Args};

type State = usize;
type LeftT = (State, Option<char>);

#[derive(Debug)]
enum Mod {
    One,
    ZeroOrOne,
    OneOrMore,
    Any,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Op {
    And,
    Or,
    Paren,
}

impl Mod {
    fn from(c: &char) -> Option<Mod> {
        match c {
            '?' => Some(Mod::ZeroOrOne),
            '+' => Some(Mod::OneOrMore),
            '*' => Some(Mod::Any),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum PatternSection {
    And(Vec<PatternSection>, Mod),
    Or(Vec<PatternSection>, Mod),
    Char(char, Mod),
}

impl PatternSection {
    fn to_transition(&self, start: State, next: State) -> (HashMap<LeftT, State>, State) {
        let mut out = HashMap::new();
        let mut end = start;

        let (states, new_end) = self.to_transition_without_mod(start, next);
        for (k, v) in states {
            out.insert(k, v);
        }
        end = new_end;

        let m = match self {
            PatternSection::And(_, m) => m,
            PatternSection::Or(_, m) => m,
            PatternSection::Char(_, m) => m,
        };
        match m {
            Mod::One => {}
            Mod::ZeroOrOne => {
                out.insert((start, None), end);
            }
            Mod::OneOrMore => unimplemented!(),
            Mod::Any => {
                // Todo: Avoid unnecessary extension.
                out.insert((end, None), start);
                out.insert((start, None), end + 1);
                end += 1;
            }
        }

        (out, end)
    }

    fn to_transition_without_mod(
        &self,
        start: State,
        next: State,
    ) -> (HashMap<LeftT, State>, State) {
        match self {
            PatternSection::And(list, m) => self.to_transition_and(list, start, next),
            PatternSection::Or(list, m) => self.to_transition_or(list, start, next),
            PatternSection::Char(c, m) => self.to_transition_char(*c, start, next),
        }
    }

    fn to_transition_char(
        &self,
        c: char,
        start: State,
        next: State,
    ) -> (HashMap<LeftT, State>, State) {
        let mut out = HashMap::new();
        let mut end = start;

        out.insert((end, Some(c)), next);
        end = next;

        (out, end)
    }

    fn to_transition_and(
        &self,
        list: &Vec<PatternSection>,
        start: State,
        next: State,
    ) -> (HashMap<LeftT, State>, State) {
        let mut out = HashMap::new();
        let mut end = start;

        for section in list {
            let (states, new_end) = section.to_transition(end, end + 1);
            for (k, v) in states {
                out.insert(k, v);
            }
            end = new_end;
        }

        (out, end)
    }

    fn to_transition_or(
        &self,
        list: &Vec<PatternSection>,
        start: State,
        next: State,
    ) -> (HashMap<LeftT, State>, State) {
        let mut out = HashMap::new();
        let mut end = start;
        let mut latest_end = start;
        let mut ends = vec![];

        for section in list {
            let (states, new_end) = section.to_transition(start, latest_end + 1);
            ends.push(new_end);
            latest_end = new_end;
            for (k, v) in states {
                out.insert(k, v);
            }
        }
        // Todo: figure out how to skip the +1 last transition.
        for prev_end in ends {
            out.insert((prev_end, None), latest_end + 1);
        }
        end = latest_end + 1;

        (out, end)
    }
}

#[derive(Debug)]
struct Engine {
    transitions: HashMap<LeftT, State>,
    finish_state: State,
}

impl Engine {
    fn new(pattern: &str) -> Engine {
        let pattern = Engine::parse(pattern);
        let (transitions, finish_state) = pattern.to_transition(0, 1);
        Engine {
            transitions,
            finish_state,
        }
    }

    fn parse(raw: &str) -> PatternSection {
        let mut stack: Vec<PatternSection> = vec![];
        let mut ops: Vec<Op> = vec![];

        let mut need_and = false;
        let mut idx = 0usize;

        for c in raw.chars() {
            if let Some(pattern_mod) = Mod::from(&c) {
                Engine::inject_mod(&mut stack, pattern_mod);
            } else if c == '|' {
                Engine::collapse_stacks(&mut stack, &mut ops, |op| match op {
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
                Engine::collapse_stacks(&mut stack, &mut ops, |op| match op {
                    Some(Op::Paren) => true,
                    _ => false,
                });
                assert_eq!(Some(Op::Paren), ops.pop());
                if idx < raw.len() - 1 {
                    need_and = true;
                }
            } else if c.is_ascii_alphabetic() {
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

        Engine::collapse_stacks(&mut stack, &mut ops, |op| match op {
            None => true,
            Some(_) => false,
        });
        assert!(ops.is_empty());
        assert_eq!(1, stack.len());

        stack.pop().unwrap()
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

            let (op, count) = Engine::pop_same(ops);
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

    fn dump_dot(&self) {
        println!("digraph {{");

        for (k, v) in &self.transitions {
            println!("\t{} -> {}[label=\"{}\"]", k.0, v, k.1.unwrap_or(' '));
        }

        println!("}}");
    }
}

fn main() {
    let mut args = std::env::args();
    if args.len() != 2 {
        panic!("Missing arguments. Call: ./bin PATTERN");
    }

    let eng = Engine::new(args.nth(1).expect("Missing argument").as_str());
    eng.dump_dot();
}
