/// Ultra-fast, zero-dependency inline arithmetic expression evaluator for view-launcher.
/// Supports +, -, *, /, %, ^, parentheses, floats, integers, hex (0x...), and bin (0b...).

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Power,
    LParen,
    RParen,
    Sqrt,
    Abs,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    has_operator: bool,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            has_operator: false,
        }
    }

    fn tokenize(&mut self) -> Option<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(&c) = self.chars.peek() {
            match c {
                ' ' | '\t' | '\r' | '\n' => {
                    self.chars.next();
                }
                '+' => {
                    tokens.push(Token::Plus);
                    self.has_operator = true;
                    self.chars.next();
                }
                '-' => {
                    tokens.push(Token::Minus);
                    self.has_operator = true;
                    self.chars.next();
                }
                '*' | 'x' | 'X' | '×' => {
                    tokens.push(Token::Multiply);
                    self.has_operator = true;
                    self.chars.next();
                }
                '/' | '÷' => {
                    tokens.push(Token::Divide);
                    self.has_operator = true;
                    self.chars.next();
                }
                '%' => {
                    tokens.push(Token::Modulo);
                    self.has_operator = true;
                    self.chars.next();
                }
                '^' => {
                    tokens.push(Token::Power);
                    self.has_operator = true;
                    self.chars.next();
                }
                '(' => {
                    tokens.push(Token::LParen);
                    self.chars.next();
                }
                ')' => {
                    tokens.push(Token::RParen);
                    self.chars.next();
                }
                '0'..='9' | '.' => {
                    let num = self.read_number()?;
                    tokens.push(Token::Number(num));
                }
                'a'..='z' | 'A'..='Z' => {
                    let ident = self.read_ident();
                    match ident.to_lowercase().as_str() {
                        "sqrt" => {
                            tokens.push(Token::Sqrt);
                            self.has_operator = true;
                        }
                        "abs" => {
                            tokens.push(Token::Abs);
                            self.has_operator = true;
                        }
                        "pi" => {
                            tokens.push(Token::Number(std::f64::consts::PI));
                        }
                        "e" => {
                            tokens.push(Token::Number(std::f64::consts::E));
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            }
        }

        // Must contain at least one operator or math function to be treated as a calculator query
        if !self.has_operator || tokens.is_empty() {
            return None;
        }

        Some(tokens)
    }

    fn read_number(&mut self) -> Option<f64> {
        let mut s = String::new();
        
        // Check for 0x (hex) or 0b (bin)
        if let Some(&'0') = self.chars.peek() {
            s.push(self.chars.next().unwrap());
            if let Some(&c) = self.chars.peek() {
                if c == 'x' || c == 'X' {
                    s.clear();
                    self.chars.next(); // consume x
                    while let Some(&h) = self.chars.peek() {
                        if h.is_ascii_hexdigit() {
                            s.push(self.chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if s.is_empty() { return None; }
                    return i64::from_str_radix(&s, 16).ok().map(|v| v as f64);
                } else if c == 'b' || c == 'B' {
                    s.clear();
                    self.chars.next(); // consume b
                    while let Some(&b) = self.chars.peek() {
                        if b == '0' || b == '1' {
                            s.push(self.chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if s.is_empty() { return None; }
                    return i64::from_str_radix(&s, 2).ok().map(|v| v as f64);
                }
            }
        }

        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }

        s.parse::<f64>().ok()
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_alphabetic() {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        s
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse(&mut self) -> Option<f64> {
        let result = self.parse_expr()?;
        if self.pos == self.tokens.len() {
            Some(result)
        } else {
            None
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_expr(&mut self) -> Option<f64> {
        let mut left = self.parse_term()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    let right = self.parse_term()?;
                    left += right;
                }
                Token::Minus => {
                    self.next();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }

        Some(left)
    }

    fn parse_term(&mut self) -> Option<f64> {
        let mut left = self.parse_power()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Multiply => {
                    self.next();
                    let right = self.parse_power()?;
                    left *= right;
                }
                Token::Divide => {
                    self.next();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return None; // Division by zero
                    }
                    left /= right;
                }
                Token::Modulo => {
                    self.next();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return None;
                    }
                    left %= right;
                }
                _ => break,
            }
        }

        Some(left)
    }

    fn parse_power(&mut self) -> Option<f64> {
        let left = self.parse_factor()?;

        if let Some(Token::Power) = self.peek() {
            self.next();
            let right = self.parse_power()?;
            Some(left.powf(right))
        } else {
            Some(left)
        }
    }

    fn parse_factor(&mut self) -> Option<f64> {
        match self.peek()? {
            Token::Number(val) => {
                let v = *val;
                self.next();
                Some(v)
            }
            Token::Minus => {
                self.next();
                let val = self.parse_factor()?;
                Some(-val)
            }
            Token::Plus => {
                self.next();
                self.parse_factor()
            }
            Token::LParen => {
                self.next();
                let val = self.parse_expr()?;
                if let Some(Token::RParen) = self.next() {
                    Some(val)
                } else {
                    None
                }
            }
            Token::Sqrt => {
                self.next();
                let val = self.parse_factor()?;
                if val < 0.0 {
                    None
                } else {
                    Some(val.sqrt())
                }
            }
            Token::Abs => {
                self.next();
                let val = self.parse_factor()?;
                Some(val.abs())
            }
            _ => None,
        }
    }
}

/// Evaluates a user query string as an arithmetic expression.
/// Returns `Some(f64)` if it is a valid mathematical expression, otherwise `None`.
pub fn evaluate(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut lexer = Lexer::new(trimmed);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let result = parser.parse()?;

    if result.is_finite() {
        Some(result)
    } else {
        None
    }
}

/// Formats a float calculation result cleanly with thousand separators for integers.
pub fn format_result(val: f64) -> String {
    if val.fract() == 0.0 && val.abs() < 1e15 {
        let int_val = val as i64;
        let s = int_val.to_string();
        let is_negative = s.starts_with('-');
        let digits = if is_negative { &s[1..] } else { &s };
        
        let mut formatted = String::new();
        let len = digits.len();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                formatted.push(',');
            }
            formatted.push(c);
        }
        if is_negative {
            format!("-{}", formatted)
        } else {
            formatted
        }
    } else {
        // Truncate trailing floating zeros
        let s = format!("{:.8}", val);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(evaluate("2 + 2"), Some(4.0));
        assert_eq!(evaluate("10 - 3 * 2"), Some(4.0));
        assert_eq!(evaluate("(10 - 3) * 2"), Some(14.0));
        assert_eq!(evaluate("100 / 4"), Some(25.0));
        assert_eq!(evaluate("2 ^ 10"), Some(1024.0));
        assert_eq!(evaluate("10 % 3"), Some(1.0));
    }

    #[test]
    fn test_hex_and_bin() {
        assert_eq!(evaluate("0x10 + 5"), Some(21.0));
        assert_eq!(evaluate("0b1010 + 2"), Some(12.0));
    }

    #[test]
    fn test_formatting() {
        assert_eq!(format_result(1024.0), "1,024");
        assert_eq!(format_result(1000000.0), "1,000,000");
        assert_eq!(format_result(3.14159), "3.14159");
    }

    #[test]
    fn test_non_math_queries() {
        assert_eq!(evaluate("firefox"), None);
        assert_eq!(evaluate("123"), None);
        assert_eq!(evaluate("code /home"), None);
    }
}
