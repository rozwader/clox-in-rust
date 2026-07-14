use std::{cell::RefCell, fmt, rc::Rc};

use crate::_chunk::Chunk;

#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    String(String),
    Function(Function),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: Option<String>
}

pub enum FunctionType {
    Function,
    Script
}

impl Function {
    pub fn new(chunk: Chunk) -> Self {
        Self {
            arity: 0,
            name: None,
            chunk
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "<fn {}>", name),
            None => write!(f, "<script>")
        }
    }
}

pub type ObjRef = Rc<RefCell<Obj>>;