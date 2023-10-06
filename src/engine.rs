use crate::parser::*;
use crate::types::*;

#[derive(Debug)]
pub struct Engine {
    transitions: Transition,
    finish_state: State,
}

impl Engine {
    pub fn new(pattern: &str) -> Engine {
        let pattern = Parser::parse(pattern);
        let (transitions, finish_state) = pattern.to_transition(0, 1);
        Engine {
            transitions,
            finish_state,
        }
    }

    pub fn is_match(&self, s: &str) -> bool {
        let mut stack: Vec<(State, usize)> = vec![(0, 0)];
        let chars = s.chars().collect::<Vec<_>>();

        while let Some((state, i)) = stack.pop() {
            if state == self.finish_state && i >= chars.len() {
                return true;
            }

            let mut new_states = self.transitions.states_from(state, chars.get(i), i);
            stack.append(&mut new_states);
        }

        false
    }

    pub fn dump_dot(&self) {
        println!("digraph {{");
        println!("\tStart [color=\"blue\"]");
        println!("\tFinish [color=\"green\"]");

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

        for (k, vs) in &self.transitions.base {
            for v in vs {
                println!(
                    "\t{} -> {}[label=\"{}\"]",
                    to_label(k.0),
                    to_label(*v),
                    k.1.unwrap_or(' ')
                );
            }
        }

        for (k, vs) in &self.transitions.negated {
            for v in vs {
                println!(
                    "\t{} -> {}[label=\"^{}\",color=\"purple\"]",
                    to_label(k.0),
                    to_label(*v),
                    k.1.iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join("")
                );
            }
        }

        println!("}}");
    }
}

#[cfg(test)]
mod test {
    use crate::engine::*;

    #[test]
    fn test_empty() {
        assert!(Engine::new("").is_match(""));
        assert!(!Engine::new("").is_match("a"));
        assert!(!Engine::new("").is_match("abc"));
    }

    #[test]
    fn test_paren() {
        assert!(Engine::new("a(a)a").is_match("aaa"));
        assert!(Engine::new("aa(a)").is_match("aaa"));
        assert!(Engine::new("(aa)a").is_match("aaa"));

        assert!(!Engine::new("a(a)a").is_match("aaaa"));
        assert!(!Engine::new("aa(a)").is_match("aaaa"));
        assert!(!Engine::new("(aa)a").is_match("aaaa"));

        assert!(!Engine::new("a(a)a").is_match("aa"));
        assert!(!Engine::new("aa(a)").is_match("aa"));
        assert!(!Engine::new("(aa)a").is_match("aa"));
    }

    #[test]
    fn test_or() {
        assert!(Engine::new("a|b").is_match("a"));
        assert!(Engine::new("a|b").is_match("b"));

        assert!(!Engine::new("a|b").is_match("ba"));
        assert!(!Engine::new("a|b").is_match("ab"));
        assert!(!Engine::new("a|b").is_match(""));
    }

    #[test]
    fn test_mod_any() {
        assert!(Engine::new("a*").is_match(""));
        assert!(Engine::new("a*").is_match("a"));
        assert!(Engine::new("a*").is_match("aaaaaaaaaaaaaaaaaaaaaa"));

        assert!(!Engine::new("a*").is_match("aaaab"));

        assert!(Engine::new("(aaa)*").is_match(""));
        assert!(Engine::new("(aaa)*").is_match("aaa"));
        assert!(Engine::new("(aaa)*").is_match("aaaaaa"));

        assert!(!Engine::new("(aaa)*").is_match("a"));
        assert!(!Engine::new("(aaa)*").is_match("aa"));
    }

    #[test]
    fn test_mod_one_or_more() {
        assert!(Engine::new("a+").is_match("a"));
        assert!(Engine::new("a+").is_match("aaaa"));

        assert!(!Engine::new("a+").is_match(""));
        assert!(!Engine::new("a+").is_match("b"));
        assert!(!Engine::new("a+").is_match("aab"));

        assert!(Engine::new("(aaa)+").is_match("aaa"));
        assert!(Engine::new("(aaa)+").is_match("aaaaaaaaa"));

        assert!(!Engine::new("(aaa)+").is_match("aa"));
        assert!(!Engine::new("(aaa)+").is_match("aab"));
    }

    #[test]
    fn test_mod_zero_or_one() {
        assert!(Engine::new("a?").is_match(""));
        assert!(Engine::new("a?").is_match("a"));

        assert!(!Engine::new("a?").is_match("aaa"));
        assert!(!Engine::new("a?").is_match("b"));

        assert!(Engine::new("(aaa)?").is_match(""));
        assert!(Engine::new("(aaa)?").is_match("aaa"));

        assert!(!Engine::new("(aaa)?").is_match("a"));
        assert!(!Engine::new("(aaa)?").is_match("aa"));
        assert!(!Engine::new("(aaa)?").is_match("aab"));
    }

    #[test]
    fn test_complex() {
        assert!(Engine::new("cc?|cc").is_match("c"));

        assert!(Engine::new("a*(bb|cc?|(aaa|cd+c|d+))?").is_match(""));
        assert!(Engine::new("a*(bb|cc?|(aaa|cd+c|d+))?").is_match("aaa"));
        assert!(Engine::new("a*(bb|cc?|(aaa|cd+c|d+))?").is_match("ac"));
        assert!(Engine::new("a*(bb|cc?|(aaa|cd+c|d+))?").is_match("acc"));
        assert!(Engine::new("a*(bb|cc?|(aaa|cd+c|d+))?").is_match("acdddddc"));
    }

    #[test]
    fn test_char_group() {
        assert!(Engine::new("ab[cd]").is_match("abc"));
        assert!(Engine::new("ab[cd]").is_match("abd"));

        assert!(!Engine::new("ab[cd]").is_match("abe"));
        assert!(!Engine::new("ab[cd]").is_match("abcd"));

        assert!(Engine::new("ab[cd]*").is_match("ab"));
        assert!(Engine::new("ab[cd]*").is_match("abc"));
        assert!(Engine::new("ab[cd]*").is_match("abccccc"));
        assert!(Engine::new("ab[cd]*").is_match("abddccdccc"));

        assert!(!Engine::new("ab[cd]*").is_match("abddccdcccr"));
    }

    #[test]
    fn test_negated_char_group() {
        assert!(Engine::new("a[^bc]d").is_match("aed"));
        assert!(Engine::new("a[^bc]d").is_match("aad"));
        assert!(Engine::new("a[^bc]d").is_match("add"));

        assert!(!Engine::new("a[^bc]d").is_match("abd"));
        assert!(!Engine::new("a[^bc]d").is_match("acd"));
        assert!(!Engine::new("a[^bc]d").is_match("ad"));
    }
}
