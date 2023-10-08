use std::collections::HashMap;

pub type State = usize;
pub type LeftT = (State, Option<char>);
pub type TransitionAndEndState = (Transition, State);

#[derive(Debug, PartialEq)]
pub struct Transition {
    pub base: HashMap<LeftT, Vec<State>>,
    //                   From           NotChars   To
    pub negated: HashMap<State, HashMap<Vec<char>, Vec<State>>>,
}

impl Transition {
    pub fn new() -> Transition {
        Transition {
            base: HashMap::new(),
            negated: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: Transition) {
        for (k, mut v) in other.base {
            self.base.entry(k).or_insert(vec![]).append(&mut v);
        }

        for (k, v) in other.negated {
            let submap = self.negated.entry(k).or_insert(HashMap::new());
            for (subk, mut subv) in v {
                submap.entry(subk).or_insert(vec![]).append(&mut subv);
            }
        }
    }

    pub fn insert_base(&mut self, k: LeftT, v: State) {
        self.base.entry(k).or_insert(vec![]).push(v);
    }

    pub fn insert_negated(&mut self, state: State, not_chars: Vec<char>, to: State) {
        let submap = self.negated.entry(state).or_insert(HashMap::new());
        submap.entry(not_chars).or_insert(vec![]).push(to);
    }

    pub fn states_from(&self, state: State, c: Option<&char>, i: usize) -> Vec<(State, usize)> {
        let mut out = vec![];

        if let Some(c) = c {
            if let Some(new_states) = self.base.get(&(state, Some(*c))) {
                for new_state in new_states {
                    out.push((*new_state, i + 1));
                }
            }

            if let Some(new_states) = self.base.get(&(state, Some('.'))) {
                for new_state in new_states {
                    out.push((*new_state, i + 1));
                }
            }

            if let Some(submap) = self.negated.get(&state) {
                for (not_chars, new_states) in submap {
                    if !not_chars.contains(c) {
                        for new_state in new_states {
                            out.push((*new_state, i + 1));
                        }
                    }
                }
            }
        }

        if let Some(new_states) = self.base.get(&(state, None)) {
            for new_state in new_states {
                out.push((*new_state, i));
            }
        }

        out
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Op {
    And,
    Or,
    Paren,
}

#[derive(Debug, PartialEq)]
pub enum Mod {
    One,
    ZeroOrOne,
    OneOrMore,
    Any,
    Range(usize, usize),
}

impl Mod {
    pub fn from(c: &char) -> Option<Mod> {
        match c {
            '?' => Some(Mod::ZeroOrOne),
            '+' => Some(Mod::OneOrMore),
            '*' => Some(Mod::Any),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PatternSection {
    And(Vec<PatternSection>, Mod),
    Or(Vec<PatternSection>, Mod),
    Char(char, Mod),
    CharGroup(Vec<char>, Mod, bool), // chars + mod + is-negated
}

impl PatternSection {
    pub fn to_transition(&self, start: State, next: State) -> TransitionAndEndState {
        let mut out = Transition::new();

        let (states, new_end) = self.to_transition_without_mod(start, next);

        out.merge(states);
        let mut end = new_end;

        match self.get_mod() {
            Mod::One => {}
            Mod::ZeroOrOne => {
                out.insert_base((start, None), end);
            }
            Mod::OneOrMore => {
                out.insert_base((end, None), start);
            }
            Mod::Any => {
                out.insert_base((end, None), start);
                out.insert_base((start, None), end + 1);
                end += 1;
            }
            Mod::Range(min, max) => {
                let mut skip_list = vec![];

                assert!(*max >= 1);

                if *min == 0 {
                    skip_list.push(start);
                }

                for i in 1..*max {
                    if i >= *min {
                        skip_list.push(end);
                    }
                    let (states, new_end) = self.to_transition_without_mod(end, end + 1);
                    out.merge(states);
                    end = new_end;
                }

                for skip_state in skip_list {
                    out.insert_base((skip_state, None), end);
                }
            }
        }

        (out, end)
    }

    fn to_transition_without_mod(&self, start: State, next: State) -> TransitionAndEndState {
        match self {
            PatternSection::And(list, _) => self.to_transition_and(list, start, next),
            PatternSection::Or(list, _) => self.to_transition_or(list, start, next),
            PatternSection::Char(c, _) => self.to_transition_char(*c, start, next),
            PatternSection::CharGroup(cs, _, is_negated) => {
                self.to_transition_char_group(cs, *is_negated, start, next)
            }
        }
    }

    fn to_transition_char_group(
        &self,
        chars: &Vec<char>,
        is_negated: bool,
        start: State,
        next: State,
    ) -> TransitionAndEndState {
        let mut out = Transition::new();

        if is_negated {
            out.insert_negated(start, chars.clone(), next);
        } else {
            for c in chars {
                out.insert_base((start, Some(*c)), next);
            }
        }

        (out, next)
    }

    fn to_transition_char(&self, c: char, start: State, next: State) -> TransitionAndEndState {
        let mut out = Transition::new();
        out.insert_base((start, Some(c)), next);
        (out, next)
    }

    fn to_transition_and(
        &self,
        list: &Vec<PatternSection>,
        start: State,
        next: State,
    ) -> TransitionAndEndState {
        let mut out = Transition::new();
        let mut end = start;
        let mut new_next = next;

        for section in list {
            let (states, new_end) = section.to_transition(end, new_next);
            out.merge(states);
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
    ) -> TransitionAndEndState {
        let mut out = Transition::new();
        let mut latest_end = start;
        let mut new_next = next;
        let mut ends = vec![];

        for section in list {
            let (states, new_end) = section.to_transition(start, new_next);
            ends.push(new_end);
            latest_end = new_end;
            new_next = latest_end + 1;
            out.merge(states);
        }

        // Todo: figure out how to skip the +1 last transition.
        for prev_end in ends {
            out.insert_base((prev_end, None), latest_end + 1);
        }

        (out, latest_end + 1)
    }

    fn get_mod(&self) -> &Mod {
        match self {
            PatternSection::And(_, m) => m,
            PatternSection::Or(_, m) => m,
            PatternSection::Char(_, m) => m,
            PatternSection::CharGroup(_, m, _) => m,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parser::*;
    use crate::types::*;

    #[test]
    fn test_empty() {
        assert_eq!(transition_this(""), (TransitionBuilder::new().build(), 0));
    }

    #[test]
    fn test_and() {
        assert_eq!(
            transition_this("abc"),
            (
                TransitionBuilder::new()
                    .with_base(HashMap::from([
                        ((0, Some('a')), vec![1]),
                        ((1, Some('b')), vec![2]),
                        ((2, Some('c')), vec![3]),
                    ]))
                    .build(),
                3,
            ),
        );
    }

    #[test]
    fn test_or() {
        assert_eq!(
            transition_this("a|b.|3"),
            (
                TransitionBuilder::new()
                    .with_base(HashMap::from([
                        ((0, Some('a')), vec![1]),
                        ((0, Some('b')), vec![2]),
                        ((2, Some('.')), vec![3]),
                        ((0, Some('3')), vec![4]),
                        ((1, None), vec![5]),
                        ((3, None), vec![5]),
                        ((4, None), vec![5]),
                    ]))
                    .build(),
                5
            )
        );
    }

    #[test]
    fn test_mods() {
        assert_eq!(
            transition_this("a+"),
            (
                TransitionBuilder::new()
                    .with_base(HashMap::from([
                        ((0, Some('a')), vec![1]),
                        ((1, None), vec![0])
                    ]))
                    .build(),
                1,
            ),
        );
        assert_eq!(
            transition_this("a?"),
            (
                TransitionBuilder::new()
                    .with_base(HashMap::from([
                        ((0, Some('a')), vec![1]),
                        ((0, None), vec![1])
                    ]))
                    .build(),
                1,
            ),
        );
        assert_eq!(
            transition_this("a*"),
            (
                TransitionBuilder::new()
                    .with_base(HashMap::from([
                        ((0, Some('a')), vec![1]),
                        ((1, None), vec![0]),
                        ((0, None), vec![2])
                    ]))
                    .build(),
                2
            ),
        );
    }

    #[test]
    fn test_groups() {
        assert_eq!(
            transition_this("[ab]"),
            (
                TransitionBuilder::new()
                    .with_base(HashMap::from([
                        ((0, Some('b')), vec![1]),
                        ((0, Some('a')), vec![1]),
                    ]))
                    .build(),
                1,
            ),
        );

        assert_eq!(
            transition_this("[^ab]"),
            (
                TransitionBuilder::new()
                    .with_negated(HashMap::from([(
                        0,
                        HashMap::from([(vec!['a', 'b'], vec![1])])
                    ),]))
                    .build(),
                1,
            ),
        );
    }

    fn transition_this(raw_pattern: &str) -> TransitionAndEndState {
        let p = Parser::parse(raw_pattern);
        p.to_transition(0, 1)
    }

    struct TransitionBuilder {
        t: Transition,
    }

    impl TransitionBuilder {
        fn new() -> TransitionBuilder {
            TransitionBuilder {
                t: Transition::new(),
            }
        }

        fn with_base(
            mut self,
            base: HashMap<(State, Option<char>), Vec<State>>,
        ) -> TransitionBuilder {
            self.t.base = base;
            self
        }

        fn with_negated(
            mut self,
            negated: HashMap<State, HashMap<Vec<char>, Vec<State>>>,
        ) -> TransitionBuilder {
            self.t.negated = negated;
            self
        }

        fn build(self) -> Transition {
            self.t
        }
    }
}
