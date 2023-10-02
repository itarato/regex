# Toy Regular Expression parser

Workflow:
- gets a pattern input, for example `cargo run -- "ab?(cd|ba*)+"`
- parses the pattern into an AST of AND / OR / token
- transforms the AST into a state graph:

![state graph](./misc/graph.svg)
