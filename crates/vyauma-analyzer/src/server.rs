use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use vre_compiler::lexer::Lexer;
use vre_compiler::parser::Parser;

pub struct Backend {
    client: Client,
    document_map: Mutex<HashMap<String, String>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: Mutex::new(HashMap::new()),
        }
    }

    async fn check_document(&self, uri: Url, text: &str) {
        let lexer = Lexer::new(text);
        let mut parser = Parser::new(lexer);
        match parser.parse_program() {
            Ok(_) => {
                // No syntax errors! Clear diagnostics
                self.client.publish_diagnostics(uri, vec![], None).await;
            }
            Err(e) => {
                let diag = Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 1 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Syntax Error: {:?}", e),
                    ..Default::default()
                };
                self.client.publish_diagnostics(uri, vec![diag], None).await;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "vyauma-analyzer initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        
        self.document_map
            .lock()
            .unwrap()
            .insert(uri.to_string(), text.clone());

        self.check_document(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().next() {
            let text = change.text;
            self.document_map
                .lock()
                .unwrap()
                .insert(uri.to_string(), text.clone());

            self.check_document(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.document_map.lock().unwrap().remove(&uri.to_string());
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}
