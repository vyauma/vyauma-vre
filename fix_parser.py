import re

def fix_parser(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Replacements
    content = content.replace('self.current_token == TokenKind::', 'self.current_token.kind == TokenKind::')
    content = content.replace('self.current_token != TokenKind::', 'self.current_token.kind != TokenKind::')
    content = content.replace('self.peek_token == TokenKind::', 'self.peek_token.kind == TokenKind::')
    content = content.replace('self.peek_token != TokenKind::', 'self.peek_token.kind != TokenKind::')
    
    content = content.replace('match &self.current_token {', 'match &self.current_token.kind {')
    content = content.replace('match self.current_token {', 'match &self.current_token.kind {')
    content = content.replace('match &self.peek_token {', 'match &self.peek_token.kind {')
    
    content = content.replace('fn expect_peek(&mut self, token: Token)', 'fn expect_peek(&mut self, token: TokenKind)')
    content = content.replace('if self.peek_token == token {', 'if self.peek_token.kind == token {')
    
    content = content.replace('match token {', 'match &token.kind {')
    
    # In parse_statement, `match self.current_token` was replaced by `match &self.current_token.kind`.
    # Wait, some TokenKind values might be cloned. It's safe since TokenKind derives Clone.
    
    # Fix if let TokenKind::Identifier(id) = &self.current_token
    content = content.replace('if let TokenKind::Identifier(id) = &self.current_token {', 'if let TokenKind::Identifier(id) = &self.current_token.kind {')
    content = content.replace('if let TokenKind::Identifier(id) = &self.peek_token {', 'if let TokenKind::Identifier(id) = &self.peek_token.kind {')

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

fix_parser('crates/vre-compiler/src/parser.rs')
fix_parser('crates/vre-compiler/src/parser_indent.rs')
