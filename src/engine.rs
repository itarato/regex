use std::collections::HashMap;

use crate::parser::*;
use crate::types::*;

#[derive(Debug)]
pub struct Engine {
    transitions: HashMap<LeftT, State>,
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

        for (k, v) in &self.transitions {
            println!(
                "\t{} -> {}[label=\"{}\"]",
                to_label(k.0),
                to_label(*v),
                k.1.unwrap_or(' ')
            );
        }

        println!("}}");
    }
}
