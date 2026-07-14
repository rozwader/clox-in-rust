use std::{cell::RefCell, process, rc::Rc};

use crate::{_chunk::{Chunk, OpCode, Value}, _object::{Function, Obj}, _parser::{Compiler, Parser}, _scanner::{Scanner, TokenType}, _table::Table, _vm::InterpretResult::{InterpretOk, InterpretRuntimeError}};

#[derive(PartialEq)]
pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

pub struct VM {
    pub ip: usize,
    pub stack: Vec<Value>,
    pub globals: Table,
    pub strings: Table,
    pub frames: Vec<CallFrame>
}

pub struct CallFrame {
    pub function: Rc<RefCell<Function>>,
    pub ip: usize,
    pub slots: Vec<Value>
}

impl VM {
    pub fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::new(),
            strings: Table::new(),
            globals: Table::new(),
            frames: Vec::new(),
        }
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let mut chunk = Chunk::new();

        if !compile(source, &mut chunk) {
            return InterpretResult::InterpretCompileError;
        }

        self.ip = 0;

        let result = interpret_run(self, &chunk);

        result
    }

    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    fn peek(&self, distance: usize) -> &Value {
        let index = self.stack.len() - 1 - distance;
        &self.stack[index]
    }

    pub fn read_byte(&mut self, chunk: &Chunk) -> u8 {
        let byte = chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    pub fn read_constant(&mut self, chunk: &Chunk) -> Value {
        let index = self.read_byte(chunk) as usize;
        chunk.constants.values[index].clone()
    }

    pub fn read_string(&mut self, chunk: &Chunk) -> Rc<RefCell<Obj>> {
        match self.read_constant(chunk) {
            Value::Obj(obj) => obj,
            _ => panic!("Constant is not a string"),
        }
    }

    pub fn read_global_name(&mut self, chunk: &Chunk) -> String {
        match self.read_constant(chunk) {
            Value::Obj(obj) => {
                match &*obj.borrow() {
                    Obj::String(s) => s.clone(),
                    _ => panic!("Global name must be a string"),
                }
            },
            _ => panic!("Global name must be an object"),
        }
    }

    pub fn runtime_error(&mut self, message: &str, chunk: &Chunk) {
        print!("{message} ");
        let instruction = self.ip - chunk.code.len() - 1;
        let line = chunk.lines[instruction];
        println!("[line] in script {line}\n");
        self.stack.clear();
    }

    pub fn binary_op<F>(&mut self, op: F) -> InterpretResult
    where
        F: Fn(f32, f32) -> f32,
    {
        let b = self.pop().unwrap();
        let a = self.pop().unwrap();

        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                self.push(Value::Number(op(a, b)));
                InterpretResult::InterpretOk
            },
            _ => InterpretResult::InterpretRuntimeError
        }
    }

    pub fn read_short(&mut self, chunk: &Chunk) -> u16 {
        let high = self.read_byte(chunk) as u16;
        let low = self.read_byte(chunk) as u16;

        (high << 8) | low
    }

    pub fn compare_op<F>(&mut self, op: F) -> InterpretResult
    where
        F: Fn(f32, f32) -> bool,
    {
        let b = self.pop().unwrap();
        let a = self.pop().unwrap();

        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                self.push(Value::Boolean(op(a, b)));
                InterpretResult::InterpretOk
            },
            _ => InterpretResult::InterpretRuntimeError
        }
    }
    
}

pub fn interpret_run(vm: &mut VM, chunk: &Chunk) -> InterpretResult {
    loop {
        if vm.ip >= chunk.code.len() {
            return InterpretResult::InterpretRuntimeError
        }

        let instruction = vm.read_byte(chunk);

        match OpCode::try_from(instruction) {
            Ok(OpCode::Return) => {
                return InterpretResult::InterpretOk;
            }

            Ok(OpCode::Constant) => {
                let constant = vm.read_constant(chunk);
                vm.push(constant);
            }

            Ok(OpCode::Nil) => {
                vm.push(Value::Nil)
            }

            Ok(OpCode::True) => {
                vm.push(Value::Boolean(true))
            }

            Ok(OpCode::False) => {
                vm.push(Value::Boolean(false))
            }

            Ok(OpCode::Equal) => {
                let b = vm.pop().unwrap();
                let a = vm.pop().unwrap();
                vm.push(Value::Boolean(a == b));
            }

            Ok(OpCode::Greater) => {
                if vm.compare_op(|a, b| a > b) == InterpretResult::InterpretRuntimeError {
                    return InterpretResult::InterpretRuntimeError
                }
            }

            Ok(OpCode::Less) => {
                if vm.compare_op(|a, b| a < b) == InterpretResult::InterpretRuntimeError {
                    return InterpretResult::InterpretRuntimeError
                }
            }

            Ok(OpCode::Add) => {
                let a = vm.pop().unwrap();
                let b = vm.pop().unwrap();

                match (a, b) {
                    (Value::Obj(a), Value::Obj(b)) => {
                        let a = a.borrow();
                        let b = b.borrow();
                        match (&*a, &*b) {
                            (Obj::String(a), Obj::String(b)) => {
                                let result = format!("{}{}", b, a);

                                vm.push(Value::Obj(Rc::new(RefCell::new(
                                    Obj::String(result)
                                ))))
                            },
                            _ => {
                                vm.runtime_error("Operands must be two numbers or two strings", chunk);
                                return InterpretResult::InterpretRuntimeError
                            }
                        }
                    },
                    (Value::Number(a), Value::Number(b)) => {
                        vm.push(Value::Number(a + b))
                    },
                    _ => {
                        vm.runtime_error("Operands must be two numbers or two strings", chunk);
                        return InterpretResult::InterpretRuntimeError
                    }
                }
                
            }

            Ok(OpCode::Subtract) => {
                if vm.binary_op(|a, b| a - b) == InterpretResult::InterpretRuntimeError {
                    return InterpretResult::InterpretRuntimeError
                };
            }

            Ok(OpCode::Multiply) => {
                if vm.binary_op(|a, b| a * b) == InterpretResult::InterpretRuntimeError {
                    return InterpretResult::InterpretRuntimeError
                };
            }

            Ok(OpCode::Divide) => {
                if vm.binary_op(|a, b| a / b) == InterpretResult::InterpretRuntimeError {
                    return InterpretResult::InterpretRuntimeError
                };
            }

            Ok(OpCode::Not) => {
                let value = is_falsey(&vm.pop().unwrap());
                vm.push(Value::Boolean(value));
            }

            Ok(OpCode::Negate) => {
                let value = vm.pop().unwrap();
                match value {
                    Value::Number(value) => {
                        vm.push(Value::Number(-value))
                    },
                    _ => {
                        vm.runtime_error("Operand must be a number", chunk);
                        return InterpretResult::InterpretRuntimeError
                    }
                }
            }

            Ok(OpCode::Print) => {
                let value = vm.pop().unwrap();
                println!("{}", value)
            }

            Ok(OpCode::Pop) => {
                vm.pop();
            }

            Ok(OpCode::DefineGlobal) => {
                let name = vm.read_global_name(chunk);

                vm.globals.insert(name, vm.peek(0).clone());
                vm.pop();
            }

            Ok(OpCode::GetGlobal) => {
                let name = vm.read_global_name(chunk);

                match vm.globals.get(&name) {
                    Some(value) => {
                        vm.push(value.clone());
                    }
                    None => {
                        vm.runtime_error(&format!("Undefined variable '{name}'"), chunk);
                        return InterpretResult::InterpretRuntimeError
                    }
                }
            }

            Ok(OpCode::SetGlobal) => {
                let name = vm.read_global_name(chunk);

                if vm.globals.contains_key(&name) {
                    vm.globals.insert(name, vm.peek(0).clone());
                } else {
                    vm.runtime_error(
                        &format!("Undefined variable '{}'.", name),
                        chunk
                    );
                    return InterpretResult::InterpretRuntimeError;
                }
            }

            Ok(OpCode::GetLocal) => {
                let slot = vm.read_byte(chunk) as usize;
                vm.push(vm.stack[slot].clone());
            }

            Ok(OpCode::SetLocal) => {
                let slot = vm.read_byte(chunk) as usize;
                vm.stack[slot] = vm.peek(0).clone();
            }

            Ok(OpCode::JumpIfFalse) => {
                let offset = vm.read_short(chunk);
                if is_falsey(vm.peek(0)) {
                    vm.ip += offset as usize;
                }
            }

            Ok(OpCode::Jump) => {
                let offset = vm.read_short(chunk);
                vm.ip += offset as usize;
            }

            Ok(OpCode::Loop) => {
                let offset = vm.read_short(chunk);
                vm.ip -= offset as usize;
            }

            Err(_) => {
                return InterpretResult::InterpretRuntimeError;
            }
        }
    }
}

pub fn is_falsey(value: &Value) -> bool {
    match value {
        Value::Nil => true,
        Value::Boolean(value) => {
            !value
        },
        _ => false,
    }
}

pub fn compile(source: &str, chunk: &mut Chunk) -> bool {
    let scanner = Scanner::new(source);
    let compiler = Compiler::new();
    let mut parser = Parser::new(scanner, compiler);

    parser.advance();

    while !parser.match_token(TokenType::Eof) {
        parser.declaration(chunk);
    }

    parser.end_compiler(chunk);

    return !parser.had_error;
}