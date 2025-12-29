//! Lexical analysis for Solisp
//!
//! Converts source text into a stream of tokens using LISP-style S-expressions.

mod sexpr_scanner;
mod token;

pub use sexpr_scanner::SExprScanner;
pub use token::{Token, TokenKind};
