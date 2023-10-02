use std::collections::HashMap;

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

        let (states, new_end) = self.to_transition_without_mod(start, next);
        for (k, v) in states {
            out.insert(k, v);
        }
        let mut end = new_end;

        match self.get_mod() {
            Mod::One => {}
            Mod::ZeroOrOne => {
                out.insert((start, None), end);
            }
            Mod::OneOrMore => {
                out.insert((end, None), start);
            }
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
            PatternSection::And(list, _) => self.to_transition_and(list, start, next),
            PatternSection::Or(list, _) => self.to_transition_or(list, start, next),
            PatternSection::Char(c, _) => self.to_transition_char(*c, start, next),
        }
    }

    fn to_transition_char(
        &self,
        c: char,
        start: State,
        next: State,
    ) -> (HashMap<LeftT, State>, State) {
        let mut out = HashMap::new();
        out.insert((start, Some(c)), next);
        (out, next)
    }

    fn to_transition_and(
        &self,
        list: &Vec<PatternSection>,
        start: State,
        next: State,
    ) -> (HashMap<LeftT, State>, State) {
        let mut out = HashMap::new();
        let mut end = start;
        let mut new_next = next;

        for section in list {
            let (states, new_end) = section.to_transition(end, new_next);
            for (k, v) in states {
                out.insert(k, v);
            }
            end = new_end;
            new_next = end + 1;
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
        let mut latest_end = start;
        let mut new_next = next;
        let mut ends = vec![];

        for section in list {
            let (states, new_end) = section.to_transition(start, new_next);
            ends.push(new_end);
            latest_end = new_end;
            new_next = latest_end + 1;
            for (k, v) in states {
                out.insert(k, v);
            }
        }

        // Todo: figure out how to skip the +1 last transition.
        for prev_end in ends {
            out.insert((prev_end, None), latest_end + 1);
        }

        (out, latest_end + 1)
    }

    fn get_mod(&self) -> &Mod {
        match self {
            PatternSection::And(_, m) => m,
            PatternSection::Or(_, m) => m,
            PatternSection::Char(_, m) => m,
        }
    }
}

struct Parser;

impl Parser {
    fn parse(raw: &str) -> PatternSection {
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

        Parser::collapse_stacks(&mut stack, &mut ops, |op| match op {
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

#[derive(Debug)]
struct Engine {
    transitions: HashMap<LeftT, State>,
    finish_state: State,
}

impl Engine {
    fn new(pattern: &str) -> Engine {
        let pattern = Parser::parse(pattern);
        let (transitions, finish_state) = pattern.to_transition(0, 1);
        Engine {
            transitions,
            finish_state,
        }
    }

    fn is_match(&self, s: &str) -> bool {
        let mut stack: Vec<(State, usize)> = vec![(0, 0)];
        let chars = s.chars().collect::<Vec<_>>();

        while let Some((state, i)) = stack.pop() {
            if i < chars.len() {
                if let Some(new_state) = self.transitions.get(&(state, Some(chars[i]))) {
                    if new_state == &self.finish_state && i == chars.len() - 1 {
                        return true;
                    }
                    stack.push((*new_state, i + 1));
                }
            }
            if let Some(new_state) = self.transitions.get(&(state, None)) {
                if new_state == &self.finish_state && i >= chars.len() {
                    return true;
                }
                stack.push((*new_state, i));
            }
        }

        false
    }

    fn dump_dot(&self) {
        println!("digraph {{");

        let finish = self.finish_state;
        let to_label = |s: State| {
            if s == 0 {
                "Start".into()
            } else if s == finish {
                "Finish".into()
            } else {
                format!("S{}", s)
            }
        };

        for (k, v) in &self.transitions {
            let start_label = to_label(k.0);
            let end_label = to_label(*v);
            println!(
                "\t{} -> {}[label=\"{}\"]",
                start_label,
                end_label,
                k.1.unwrap_or(' ')
            );
        }

        println!("}}");
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let eng = Engine::new(args[1].as_str());

    if args.len() == 2 {
        eng.dump_dot();
    } else if args.len() == 3 {
        dbg!(eng.is_match(args[2].as_str()));
    } else {
        panic!(
            "Invalid call with {} args. Do ./bin PATTERN or ./bin PATTERN STRING",
            args.len()
        )
    }
}
