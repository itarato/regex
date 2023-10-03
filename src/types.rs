use std::collections::HashMap;

pub type State = usize;
pub type LeftT = (State, Option<char>);

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
}

impl PatternSection {
    pub fn to_transition(&self, start: State, next: State) -> (HashMap<LeftT, State>, State) {
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
