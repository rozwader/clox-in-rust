use std::{cell::RefCell, process, rc::Rc};

use crate::{
    _chunk::{Chunk, OpCode, Value, disassemble_chunk}, _object::{Function, FunctionType, Obj}, _scanner::{Scanner, Token, TokenType},
};

pub struct Parser {
    pub current: Option<Token>,
    pub previous: Option<Token>,
    pub had_error: bool,
    pub panic_mode: bool,
    pub current_scanner: Option<Scanner>,
    pub current_compiler: Option<Compiler>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    pub fn next(self) -> Self {
        use Precedence::*;

        match self {
            None => Assignment,
            Assignment => Equality,
            Or => And,
            And => Equality,
            Equality => Comparison,
            Comparison => Term,
            Term => Factor,
            Factor => Unary,
            Unary => Call,
            Call => Primary,
            Primary => Primary,
        }
    }
}

pub type ParseFn = fn(&mut Parser, &mut Chunk, bool);

pub struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

pub struct Compiler {

    pub function_type: FunctionType,
    pub locals: Vec<Local>,
    pub scope_depth: i16,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            function_type: FunctionType::Script,
            locals: Vec::new(),
            scope_depth: 0,
        }
    }
}

pub struct Local {
    pub name: Token,
    pub depth: i16
}

impl Parser {
    pub fn new(scanner: Scanner, compiler: Compiler) -> Self {
        Self {
            current: None,
            previous: None,
            had_error: false,
            panic_mode: false,
            current_scanner: Some(scanner),
            current_compiler: Some(compiler),
        }
    }

    pub fn advance(&mut self) {
        self.previous = self.current.take();

        loop {
            self.current = Some(self.current_scanner.as_mut().unwrap().scan_token());

            if self.current.as_ref().unwrap().t_type != TokenType::Error {
                break;
            };

            self.error_at(&self.current.as_ref().unwrap().lexeme.to_string(), true)
        }
    }

    pub fn consume(&mut self, t_type: TokenType, message: &str) {
        if self.current.as_ref().unwrap().t_type == t_type {
            self.advance();
            return;
        }

        self.error_at(message, true);
    }

    pub fn error_at(&mut self, message: &str, is_current: bool) {
        if self.panic_mode {
            return;
        };
        self.panic_mode = true;

        let token: &Token;
        if is_current {
            token = self.current.as_ref().unwrap();
        } else {
            token = self.previous.as_ref().unwrap();
        };
        println!("[{}] Error", token.line);

        if token.t_type == TokenType::Eof {
            print!(" at end");
        } else if token.t_type == TokenType::Error {
        } else {
            print!(" at '{}' ", token.lexeme);
        }

        println!("{message}\n");
        self.had_error = true
    }

    pub fn compile_function(&mut self) -> Function {
        let mut function_chunk = Chunk::new();

        self.begin_scope();

        self.block(&mut function_chunk);

        self.end_scope(&mut function_chunk);

        self.emit_return(&mut function_chunk);

        Function {
            arity: 0,
            chunk: function_chunk,
            name: None,
        }
    }

    pub fn fun_declaration(&mut self, chunk: &mut Chunk) {
        let global = self.parse_variable("Expect function name.", chunk);

        let function = self.compile_function();

        let value = Value::Obj(
            Rc::new(RefCell::new(
                Obj::Function(function)
            ))
        );

        let constant = self.make_constant(value, chunk);

        self.define_variable(constant, chunk);
    }

    pub fn emit_byte(&mut self, byte: u8, chunk: &mut Chunk) {
        chunk.write(byte, self.previous.as_ref().unwrap().line as u32);
    }

    pub fn emit_byte_after_byte(&mut self, byte1: u8, byte2: u8, chunk: &mut Chunk) {
        
        self.emit_byte(byte1, chunk);
        self.emit_byte(byte2, chunk);
    }

    pub fn emit_return(&mut self, chunk: &mut Chunk) {
        self.emit_byte(OpCode::Return as u8, chunk);
    }

    pub fn end_compiler(&mut self, chunk: &mut Chunk) {
        self.emit_return(chunk);
    }

    pub fn make_number(&mut self, chunk: &mut Chunk, can_assign: bool) {
        let value: f32 = self
            .previous
            .as_ref()
            .unwrap()
            .lexeme
            .parse::<f32>()
            .unwrap() as f32;
        self.emit_constant(Value::Number(value), chunk);
    }

    pub fn or_(&mut self,  chunk: &mut Chunk, can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse as u8, chunk);
        let end_jump = self.emit_jump(OpCode::Jump as u8, chunk);

        self.patch_jump(else_jump, chunk);
        self.emit_byte(OpCode::Pop as u8, chunk);

        self.parse_precedence(Precedence::Or, chunk);
        self.patch_jump(end_jump, chunk);
    }

    pub fn emit_constant(&mut self, value: Value, chunk: &mut Chunk) {
        let constant = self.make_constant(value, chunk);
        self.emit_byte_after_byte(OpCode::Constant as u8, constant, chunk);
    }

    pub fn make_constant(&mut self, value: Value, chunk: &mut Chunk) -> u8 {
        let constant = chunk.add_constant(value);
        if constant > u8::MAX.into() {
            self.error_at("Too many constants in one chunk.", false);
            return 0;
        }

        constant as u8
    }

    pub fn grouping(&mut self, chunk: &mut Chunk, can_assign: bool) {
        self.expression(chunk);
        self.consume(TokenType::RightParen, "Expected ')' after expression.");
    }

    pub fn unary(&mut self, chunk: &mut Chunk, can_assign: bool) {
        let operator_type = self.previous.as_ref().unwrap().t_type;

        self.parse_precedence(Precedence::Unary, chunk);

        match operator_type {
            TokenType::Bang => self.emit_byte(OpCode::Not as u8, chunk),
            TokenType::Minus => self.emit_byte(OpCode::Negate as u8, chunk),
            _ => return,
        }
    }

    pub fn literal(&mut self, chunk: &mut Chunk, can_assign: bool) {
        match self.previous.as_ref().unwrap().t_type {
            TokenType::False => self.emit_byte(OpCode::False as u8, chunk),
            TokenType::Nil => self.emit_byte(OpCode::Nil as u8, chunk),
            TokenType::True => self.emit_byte(OpCode::True as u8, chunk),
            _ => return,
        }
    }

    pub fn parse_precedence(&mut self, precedence: Precedence, chunk: &mut Chunk) {
        self.advance();
        let prefix_rule = self.get_rule(self.previous.as_ref().unwrap().t_type).prefix;

        if prefix_rule.is_none() {
            self.error_at("Expected expression", false);
            return;
        }

        let can_assign = precedence <= Precedence::Assignment;
        (prefix_rule.unwrap())(self, chunk, can_assign);

        while precedence
            <= self
                .get_rule(self.current.as_ref().unwrap().t_type)
                .precedence
        {
            self.advance();
            let infix_rule = self.get_rule(self.previous.as_ref().unwrap().t_type).infix;

            (infix_rule.unwrap())(self, chunk, can_assign)
        }

        if can_assign && self.match_token(TokenType::Equal) {
            self.error_at("Invalid assignment target.", false);
        }
    }

    pub fn parse_variable(&mut self, error_message: &str, chunk: &mut Chunk) -> u8 {
        self.consume(TokenType::Identifier, error_message);

        self.declare_variable();
        if self.current_compiler.as_ref().unwrap().scope_depth > 0 { return 0; };

        self.identifier_constant(chunk)
    }

    pub fn declare_variable(&mut self) {
        if self.current_compiler.as_ref().unwrap().scope_depth == 0 {
            return;
        }

        let mut is_error = false;
        let compiler = self.current_compiler.as_ref().unwrap();

        if compiler.locals.is_empty() {
            drop(compiler);
            self.add_local();
            return;
        }

        let mut index = compiler.locals.len() - 1;
        let scope_depth = compiler.scope_depth;

        loop {
            let local = &compiler.locals[index];

            if local.depth != -1 && local.depth < scope_depth {
                break;
            }

            if self.identifiers_equal(&local.name) {
                is_error = true;
            }

            if index == 0 {
                break;
            }

            index -= 1;
        }

        drop(compiler);
        if is_error {
            self.error_at("Already a variable with this name in this scope", false);
        }
        self.add_local();
    }

    pub fn identifiers_equal(&self, b: &Token) -> bool {
        self.previous.as_ref().unwrap().lexeme == b.lexeme
    }

    pub fn add_local(&mut self) {
        let name = self.previous.as_ref().unwrap();
        let new_local: Local = Local {
            name: name.clone(),
            depth: -1
        };

        self.current_compiler.as_mut().unwrap().locals.push(new_local);
    }

    pub fn identifier_constant(&mut self, chunk: &mut Chunk) -> u8 {
        self.make_constant(Value::Obj(Rc::new(RefCell::new(Obj::String(self.previous.as_ref().unwrap().lexeme.clone())))), chunk)
    }

    pub fn expression(&mut self, chunk: &mut Chunk) {
        self.parse_precedence(Precedence::Assignment, chunk)
    }

    pub fn declaration(&mut self, chunk: &mut Chunk) {
        if self.match_token(TokenType::Var) {
            self.var_declaration(chunk);
        } else {
            self.statement(chunk);
        }

        if self.panic_mode { self.synchronize(); };
    }

    pub fn var_declaration(&mut self, chunk: &mut Chunk) {
        let global = self.parse_variable("Expect variable name.", chunk);

        if self.match_token(TokenType::Equal) {
            self.expression(chunk);
        } else {
            self.emit_byte(OpCode::Nil as u8, chunk);
        }

        self.consume(TokenType::Semicolon, "Expect ';' after variable declaration");

        self.define_variable(global, chunk);
    }

    pub fn define_variable(&mut self, global: u8, chunk: &mut Chunk) {
        if self.current_compiler.as_ref().unwrap().scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_byte_after_byte(OpCode::DefineGlobal as u8, global, chunk);
    }

    pub fn and_(&mut self, chunk: &mut Chunk, can_assign: bool) {
        let end_jump = self.emit_jump(OpCode::JumpIfFalse as u8, chunk);

        self.emit_byte(OpCode::Pop as u8, chunk);
        self.parse_precedence(Precedence::And, chunk);

        self.patch_jump(end_jump, chunk);
    }

    pub fn mark_initialized(&mut self) {
    let scope_depth = self.current_compiler.as_ref().unwrap().scope_depth;

    let compiler = self.current_compiler.as_mut().unwrap();
    compiler.locals.last_mut().unwrap().depth = scope_depth;
}

    pub fn synchronize(&mut self) {
        self.panic_mode = false;

        while self.current.as_ref().unwrap().t_type != TokenType::Eof {
            if self.previous.as_ref().unwrap().t_type == TokenType::Semicolon { return; };
            match self.current.as_ref().unwrap().t_type {
                TokenType::Class | TokenType::Fun | TokenType::Var | TokenType::For | TokenType::If | TokenType::While | TokenType::Print | TokenType::Return => {
                    return;
                },
                _ => {}
            }

            self.advance();
        }
    }

    pub fn statement(&mut self, chunk: &mut Chunk) {
        if self.match_token(TokenType::Print) {
            self.print_statement(chunk);
        } else if self.match_token(TokenType::For) {
            self.for_statement(chunk);
        } else if self.match_token(TokenType::If) {
            self.if_statement(chunk);
        } else if self.match_token(TokenType::While) {
            self.while_statement(chunk);
        } else if self.match_token(TokenType::LeftBrace) {
            self.begin_scope();
            self.block(chunk);
            self.end_scope(chunk);
        } else {
            self.expression_statement(chunk);
        }
    }

    pub fn for_statement(&mut self, chunk: &mut Chunk) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");
        if self.match_token(TokenType::Semicolon) {

        } else if self.match_token(TokenType::Var) {
            self.var_declaration(chunk);
        } else {
            self.expression_statement(chunk);
        }

        let mut loop_start = chunk.code.len();
        let mut exit_jump = None;

        if !self.match_token(TokenType::Semicolon) {
            self.expression(chunk);
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            exit_jump = Some(self.emit_jump(OpCode::JumpIfFalse as u8, chunk));
            self.emit_byte(OpCode::Pop as u8, chunk);
        }

        if !self.match_token(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump as u8, chunk);
            let increment_start = chunk.code.len();

            self.expression(chunk);
            self.emit_byte(OpCode::Pop as u8, chunk);
            self.consume(TokenType::RightParen, "Expect ')' after for clauses");

            self.emit_loop(chunk, loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump, chunk);
        }

        self.statement(chunk);
        self.emit_loop(chunk, loop_start);

        if exit_jump.is_some() {
            self.patch_jump(exit_jump.unwrap(), chunk);
            self.emit_byte(OpCode::Pop as u8, chunk);
        }

        self.end_scope(chunk);
    }

    pub fn emit_loop(&mut self, chunk: &mut Chunk, loop_start: usize) {
        self.emit_byte(OpCode::Loop as u8, chunk);

        let offset = chunk.code.len() - loop_start + 2;
        if offset > u16::MAX as usize {
            self.error_at("Loop body too large.", false);
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8, chunk);
        self.emit_byte((offset & 0xff) as u8, chunk);
    }

    pub fn while_statement(&mut self, chunk: &mut Chunk) {
        let loop_start = chunk.code.len();

        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression(chunk);
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse as u8, chunk);
        self.emit_byte(OpCode::Pop as u8, chunk);
        self.statement(chunk);
        self.emit_loop(chunk, loop_start);

        self.patch_jump(exit_jump, chunk);
        self.emit_byte(OpCode::Pop as u8, chunk);
    }

    pub fn if_statement(&mut self, chunk: &mut Chunk) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression(chunk);
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit_jump(OpCode::JumpIfFalse as u8, chunk);
        self.emit_byte(OpCode::Pop as u8, chunk);
        self.statement(chunk);

        let else_jump = self.emit_jump(OpCode::Jump as u8, chunk);

        self.patch_jump(then_jump, chunk);
        self.emit_byte(OpCode::Pop as u8, chunk);

        if (self.match_token(TokenType::Else)) {
            self.statement(chunk);
        }

        self.patch_jump(else_jump, chunk);
    }

    pub fn emit_jump(&mut self, instruction: u8, chunk: &mut Chunk) -> usize {
        self.emit_byte(instruction, chunk);
        self.emit_byte_after_byte(0xff, 0xff, chunk);
        chunk.code.len() - 2
    }

    pub fn patch_jump(&mut self, offset: usize, chunk: &mut Chunk) {
        let jump = chunk.code.len() - offset - 2;

        if jump > u16::MAX as usize {
            self.error_at("Too much code to jump over.", false);
        }

        chunk.code[offset] = ((jump >> 8) & 0xff) as u8;
        chunk.code[offset + 1] = (jump & 0xff) as u8;
    }

    pub fn block(&mut self, chunk: &mut Chunk) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration(chunk);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    pub fn check(&self, token_type: TokenType) -> bool {
        if self.current.as_ref().unwrap().t_type == TokenType::Eof {
            return token_type == TokenType::Eof;
        }

        self.current.as_ref().unwrap().t_type == token_type
    }

    pub fn begin_scope(&mut self) {
        self.current_compiler.as_mut().unwrap().scope_depth += 1;
    }

    pub fn end_scope(&mut self, chunk: &mut Chunk) {
        self.current_compiler.as_mut().unwrap().scope_depth -= 1;

        loop {
            let should_pop = {
                let compiler = self.current_compiler.as_ref().unwrap();

                !compiler.locals.is_empty()
                    && compiler.locals.last().unwrap().depth > compiler.scope_depth
            };

            if !should_pop {
                break;
            }

            self.emit_byte(OpCode::Pop as u8, chunk);

            self.current_compiler
                .as_mut()
                .unwrap()
                .locals
                .pop();
        }
    }

    pub fn match_token(&mut self, t_type: TokenType) -> bool {
        if self.current.as_ref().unwrap().t_type != t_type { return false; };
        self.advance();
        true
    }

    pub fn print_statement(&mut self, chunk: &mut Chunk) {
        self.expression(chunk);
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_byte(OpCode::Print as u8, chunk);
    }

    pub fn expression_statement(&mut self, chunk: &mut Chunk) {
        self.expression(chunk);
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_byte(OpCode::Pop as u8, chunk);
    }

    pub fn variable(&mut self, chunk: &mut Chunk, can_assign: bool) {
        self.named_variable(chunk, can_assign);
    }

    pub fn named_variable(&mut self, chunk: &mut Chunk, can_assign: bool) {
        let (get_op, set_op): (u8, u8);
        let mut arg = self.resolve_local();
        if arg.is_some() {
            get_op = OpCode::GetLocal as u8;
            set_op = OpCode::SetLocal as u8;
        } else {
            arg = Some(self.identifier_constant(chunk) as usize);
            get_op = OpCode::GetGlobal as u8;
            set_op = OpCode::SetGlobal as u8;
        }

        if self.match_token(TokenType::Equal) && can_assign {
            self.expression(chunk);
            self.emit_byte_after_byte(set_op, arg.unwrap() as u8, chunk);
        } else {
            self.emit_byte_after_byte(get_op, arg.unwrap() as u8, chunk);
        }
    }

    pub fn resolve_local(&mut self) -> Option<usize> {
        let compiler = self.current_compiler.as_ref().unwrap();

        for (index, local) in compiler.locals.iter().enumerate().rev() {
            if self.identifiers_equal(&local.name) {

                if local.depth == -1 {
                    self.error_at(
                        "Can't read local variable in its own initializer.",
                        false
                    );
                }

                return Some(index);
            }
        }

        None
    }

    pub fn binary(&mut self, chunk: &mut Chunk, can_assign: bool) {
        let operator_type = self.previous.as_ref().unwrap().t_type;
        let rule = self.get_rule(operator_type);

        self.parse_precedence(rule.precedence.next(), chunk);

        match operator_type {
            TokenType::BangEqual => {
                self.emit_byte_after_byte(OpCode::Equal as u8, OpCode::Not as u8, chunk)
            }
            TokenType::EqualEqual => self.emit_byte(OpCode::Equal as u8, chunk),
            TokenType::Greater => self.emit_byte(OpCode::Greater as u8, chunk),
            TokenType::GreaterEqual => {
                self.emit_byte_after_byte(OpCode::Less as u8, OpCode::Not as u8, chunk)
            }
            TokenType::Less => self.emit_byte(OpCode::Less as u8, chunk),
            TokenType::LessEqual => {
                self.emit_byte_after_byte(OpCode::Greater as u8, OpCode::Not as u8, chunk)
            }
            TokenType::Plus => self.emit_byte(OpCode::Add as u8, chunk),
            TokenType::Minus => self.emit_byte(OpCode::Subtract as u8, chunk),
            TokenType::Star => self.emit_byte(OpCode::Multiply as u8, chunk),
            TokenType::Slash => self.emit_byte(OpCode::Divide as u8, chunk),
            _ => return,
        };
    }

    pub fn string(&mut self, chunk: &mut Chunk, can_assign: bool) {
        let token = self.previous.as_ref().unwrap();

        let value = &token.lexeme[1..token.lexeme.len() - 1];

        let obj = Obj::String(value.to_string());

        self.emit_constant(Value::Obj(Rc::new(RefCell::new(obj))), chunk);
    }

    pub fn debug_code(&self, chunk: &Chunk) {
        if !self.had_error {
            disassemble_chunk(chunk, "code");
        }
    }

    pub fn get_rule(&mut self, token: TokenType) -> ParseRule {
        match token {
            TokenType::LeftParen => ParseRule {
                prefix: Some(Parser::grouping),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::RightParen => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::LeftBrace => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::RightBrace => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Comma => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Dot => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Minus => ParseRule {
                prefix: Some(Parser::unary),
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },

            TokenType::Plus => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },

            TokenType::Semicolon => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Slash => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Factor,
            },

            TokenType::Star => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Factor,
            },

            TokenType::Bang => ParseRule {
                prefix: Some(Parser::unary),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::BangEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Equality,
            },

            TokenType::Equal => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::EqualEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Equality,
            },

            TokenType::Greater => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },

            TokenType::GreaterEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },

            TokenType::Less => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },

            TokenType::LessEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },

            TokenType::Identifier => ParseRule {
                prefix: Some(Parser::variable),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::String => ParseRule {
                prefix: Some(Parser::string),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Number => ParseRule {
                prefix: Some(Parser::make_number),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::And => ParseRule {
                prefix: None,
                infix: Some(Parser::and_),
                precedence: Precedence::None,
            },

            TokenType::Class => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Else => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::False => ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::For => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Fun => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::If => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Nil => ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Or => ParseRule {
                prefix: None,
                infix: Some(Parser::or_),
                precedence: Precedence::None,
            },

            TokenType::Print => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Return => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Super => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::This => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::True => ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Var => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::While => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Error => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },

            TokenType::Eof => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        }
    }
}
