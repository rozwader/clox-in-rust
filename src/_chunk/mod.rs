use std::{cell::RefCell, fmt, rc::Rc};

use crate::_object::{Obj, ObjRef};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Return = 0,
    Constant = 1,
    Nil = 2,
    True = 3,
    False = 4,
    Equal = 5,
    Greater = 6,
    Less = 7,
    Add = 8,
    Subtract = 9,
    Multiply = 10,
    Divide = 11,
    Not = 12,
    Negate = 13,
    Print = 14,
    Pop = 15,
    DefineGlobal = 16,
    GetGlobal = 17,
    SetGlobal = 18,
    GetLocal = 19,
    SetLocal = 20,
    JumpIfFalse = 21,
    Jump = 22,
    Loop = 23,
}

impl TryFrom<u8> for OpCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OpCode::Return),
            1 => Ok(OpCode::Constant),
            2 => Ok(OpCode::Nil),
            3 => Ok(OpCode::True),
            4 => Ok(OpCode::False),
            5 => Ok(OpCode::Equal),
            6 => Ok(OpCode::Greater),
            7 => Ok(OpCode::Less),
            8 => Ok(OpCode::Add),
            9 => Ok(OpCode::Subtract),
            10 => Ok(OpCode::Multiply),
            11 => Ok(OpCode::Divide),
            12 => Ok(OpCode::Not),
            13 => Ok(OpCode::Negate),
            14 => Ok(OpCode::Print),
            15 => Ok(OpCode::Pop),
            16 => Ok(OpCode::DefineGlobal),
            17 => Ok(OpCode::GetGlobal),
            18 => Ok(OpCode::SetGlobal),
            19 => Ok(OpCode::GetLocal),
            20 => Ok(OpCode::SetLocal),
            21 => Ok(OpCode::JumpIfFalse),
            22 => Ok(OpCode::Jump),
            23 => Ok(OpCode::Loop),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Boolean(bool),
    Number(f32),
    Nil,
    Obj(ObjRef)
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),

            Value::Obj(obj) => {
                let obj = obj.borrow();

                match &*obj {
                    Obj::String(s) => write!(f, "{}", s),
                    Obj::Function(s) => write!(f, "{}", s),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValueArray {
    pub values: Vec<Value>,
}

impl ValueArray {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn write(&mut self, value: Value) {
        self.values.push(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray,
    pub lines: Vec<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Self { code: Vec::new(), constants: ValueArray::new(), lines: Vec::new() }
    }

    pub fn write(&mut self, byte: u8, line: u32) {
        self.code.push(byte);
        self.lines.push(line)
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        ValueArray::write(&mut self.constants, value);
        self.constants.values.len() - 1
    }
}

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {name} ==");

    let mut offset = 0;

    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        print!("   | ");
    } else {
        print!("{:04} ", chunk.lines[offset]);
    }

    let instruction = OpCode::try_from(chunk.code[offset]);

    match instruction {
        Ok(OpCode::Return) => simple_instruction("Return", offset),
        Ok(OpCode::Constant) => constant_instruction("Constant", chunk, offset),
        Ok(OpCode::Nil) => simple_instruction("Nil", offset),
        Ok(OpCode::True) => simple_instruction("True", offset),
        Ok(OpCode::False) => simple_instruction("False", offset),
        Ok(OpCode::Equal) => simple_instruction("Equal", offset),
        Ok(OpCode::Greater) => simple_instruction("Greater", offset),
        Ok(OpCode::Less) => simple_instruction("Less", offset),
        Ok(OpCode::Add) => simple_instruction("Add", offset),
        Ok(OpCode::Subtract) => simple_instruction("Subtract", offset),
        Ok(OpCode::Multiply) => simple_instruction("Multiply", offset),
        Ok(OpCode::Divide) => simple_instruction("Divide", offset),
        Ok(OpCode::Not) => simple_instruction("Not", offset),
        Ok(OpCode::Negate) => simple_instruction("Negate", offset),
        Ok(OpCode::Print) => simple_instruction("Print", offset),
        Ok(OpCode::Pop) => simple_instruction("Pop", offset),
        Ok(OpCode::DefineGlobal) => constant_instruction("Define Global", chunk, offset),
        Ok(OpCode::GetGlobal) => constant_instruction("Get Global", chunk, offset),
        Ok(OpCode::SetGlobal) => constant_instruction("Set Global", chunk, offset),
        Ok(OpCode::GetLocal) => byte_instruction("Get Local", chunk, offset),
        Ok(OpCode::SetLocal) => byte_instruction("Set Local", chunk, offset),
        Ok(OpCode::Jump) => jump_instruction("Jump", 1, chunk, offset),
        Ok(OpCode::JumpIfFalse) => jump_instruction("Jump If False", 1, chunk, offset),
        Ok(OpCode::Loop) => jump_instruction("Loop", -1, chunk, offset),
        Err(_) => {
            println!("Unknown opcode, {}", chunk.code[offset]);
            offset + 1
        }
    }
}

pub fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{name}");
    offset + 1
}

pub fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset + 1] as usize;
    print!("{name}, {constant}, ");
    println!("{}", chunk.constants.values[constant]);
    offset + 2
}

pub fn byte_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let slot = chunk.code[offset + 1];
    println!("{} {}", name, slot);
    offset + 2
}

pub fn jump_instruction(name: &str, sign: i32, chunk: &Chunk, offset: usize) -> usize {
    let jump = ((chunk.code[offset + 1] as u16) << 8)
        | chunk.code[offset + 2] as u16;

    println!(
        "{:<16} {:4} -> {}",
        name,
        offset,
        (offset as i32) + 3 + sign * (jump as i32)
    );

    offset + 3
}