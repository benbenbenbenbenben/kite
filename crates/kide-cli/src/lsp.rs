use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeActionResponse, CodeDescription, CompletionItem,
    CompletionItemKind, CompletionParams, CompletionResponse, CreateFile, CreateFileOptions,
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentChangeOperation, DocumentChanges, DocumentSymbol, DocumentSymbolParams,
    DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, Location,
    MarkupContent, MarkupKind, MessageType, NumberOrString, OneOf, Position, PrepareRenameResponse,
    PublishDiagnosticsParams, Range, ResourceOp, SemanticToken as LspSemanticToken,
    SemanticTokenType, SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, SymbolKind, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, Url, WorkspaceEdit,
};
use tower_lsp::{jsonrpc, Client, LanguageServer, LspService, Server};

pub async fn run_stdio() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        open_documents: Mutex::new(HashMap::new()),
        workspace_roots: Mutex::new(Vec::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}

struct Backend {
    client: Client,
    open_documents: Mutex<HashMap<Url, String>>,
    workspace_roots: Mutex<Vec<std::path::PathBuf>>,
}

impl Backend {
    fn set_open_document(&self, uri: Url, text: String) {
        if let Ok(mut docs) = self.open_documents.lock() {
            docs.insert(uri, text);
        }
    }

    fn remove_open_document(&self, uri: &Url) {
        if let Ok(mut docs) = self.open_documents.lock() {
            docs.remove(uri);
        }
    }

    fn snapshot_open_documents(&self) -> Vec<(Url, String)> {
        self.open_documents
            .lock()
            .map(|docs| {
                docs.iter()
                    .map(|(uri, text)| (uri.clone(), text.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn open_document_text(&self, uri: &Url) -> Option<String> {
        self.open_documents
            .lock()
            .ok()
            .and_then(|docs| docs.get(uri).cloned())
    }

    fn source_for_uri(&self, uri: &Url) -> Option<String> {
        if let Some(source) = self.open_document_text(uri) {
            return Some(source);
        }
        let path = uri.to_file_path().ok()?;
        std::fs::read_to_string(path).ok()
    }

    fn find_kide_files(&self) -> Vec<std::path::PathBuf> {
        let empty = Vec::new();
        let roots = self.workspace_roots.lock().ok();
        let roots = roots.as_deref().unwrap_or(&empty);
        let mut kide_files = Vec::new();
        for root in roots {
            Self::walk_for_kide_files(root, &mut kide_files, 0);
        }
        kide_files
    }

    fn walk_for_kide_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
        if depth > 10 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
                Self::walk_for_kide_files(&path, out, depth + 1);
            } else if path.extension().and_then(|e| e.to_str()) == Some("kide") {
                out.push(path);
            }
        }
    }

    async fn publish(&self, uri: Url, diagnostics: Vec<Diagnostic>, version: Option<i32>) {
        self.client
            .send_notification::<tower_lsp::lsp_types::notification::PublishDiagnostics>(
                PublishDiagnosticsParams {
                    uri,
                    diagnostics,
                    version,
                },
            )
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        // Capture workspace roots
        let mut roots = Vec::new();
        if let Some(folders) = &params.workspace_folders {
            for folder in folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    roots.push(path);
                }
            }
        }
        if roots.is_empty() {
            if let Some(root_uri) = &params.root_uri {
                if let Ok(path) = root_uri.to_file_path() {
                    roots.push(path);
                }
            }
        }
        if let Ok(mut wr) = self.workspace_roots.lock() {
            *wr = roots;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(tower_lsp::lsp_types::CompletionOptions::default()),
                rename_provider: Some(OneOf::Right(tower_lsp::lsp_types::RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::NAMESPACE, // 0: context
                                    SemanticTokenType::CLASS,     // 1: aggregate
                                    SemanticTokenType::FUNCTION,  // 2: command
                                    SemanticTokenType::EVENT,     // 3: invariant
                                    SemanticTokenType::PROPERTY,  // 4: field
                                    SemanticTokenType::TYPE,      // 5: type
                                    SemanticTokenType::STRING,    // 6: string
                                    SemanticTokenType::KEYWORD,   // 7: keyword
                                ],
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..SemanticTokensOptions::default()
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: tower_lsp::lsp_types::InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "kide LSP initialized")
            .await;

        // Scan workspace for all .kide files and publish diagnostics
        for path in self.find_kide_files() {
            let Ok(uri) = Url::from_file_path(&path) else {
                continue;
            };
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let diagnostics = diagnostics_for_source(&source, &uri);
            self.publish(uri, diagnostics, None).await;
        }
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.set_open_document(uri.clone(), text.clone());
        let diagnostics = diagnostics_for_source(&text, &uri);
        self.publish(uri, diagnostics, None).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            let uri = params.text_document.uri;
            let text = change.text;
            self.set_open_document(uri.clone(), text.clone());
            let diagnostics = diagnostics_for_source(&text, &uri);
            self.publish(uri, diagnostics, Some(params.text_document.version))
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.remove_open_document(&uri);
        self.publish(uri, Vec::new(), None).await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        // Re-check all open documents
        for (uri, text) in self.snapshot_open_documents() {
            let diagnostics = diagnostics_for_source(&text, &uri);
            self.publish(uri, diagnostics, None).await;
        }
        // Also re-check any workspace .kide files not currently open
        let open_uris: std::collections::HashSet<Url> = self
            .snapshot_open_documents()
            .into_iter()
            .map(|(uri, _)| uri)
            .collect();
        for path in self.find_kide_files() {
            let Ok(uri) = Url::from_file_path(&path) else {
                continue;
            };
            if open_uris.contains(&uri) {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let diagnostics = diagnostics_for_source(&source, &uri);
            self.publish(uri, diagnostics, None).await;
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(source) = self.source_for_uri(&uri) else {
            return Ok(None);
        };

        let base_dir = uri
            .to_file_path()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let Some(definition) =
            kide_core::definition_at(&source, &base_dir, position.line, position.character)
                .ok()
                .flatten()
        else {
            return Ok(None);
        };

        let Ok(target_uri) = Url::from_file_path(&definition.file_path) else {
            return Ok(None);
        };

        let range = range_from_span(definition.span);
        Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
            target_uri, range,
        ))))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(source) = self.source_for_uri(&uri) else {
            return Ok(None);
        };

        let symbols = document_symbols_for_source(&source);
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<CodeActionResponse>> {
        let actions =
            code_actions_for_diagnostics(&params.text_document.uri, &params.context.diagnostics);
        Ok((!actions.is_empty()).then_some(actions))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(source) = self.source_for_uri(&uri) else {
            return Ok(None);
        };

        let base_dir = uri
            .to_file_path()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let hover_info = kide_core::hover_at(&source, &base_dir, position.line, position.character)
            .ok()
            .flatten();

        let Some(info) = hover_info else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: info.markdown,
            }),
            range: Some(range_from_span(info.span)),
        }))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let Some(source) = self.source_for_uri(&uri) else {
            return Ok(None);
        };

        let base_dir = uri
            .to_file_path()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let completions =
            kide_core::completions_at(&source, &base_dir, position.line, position.character)
                .ok()
                .unwrap_or_default();

        if completions.is_empty() {
            return Ok(None);
        }

        let items: Vec<CompletionItem> = completions
            .into_iter()
            .map(|c| {
                let kind = match c.kind {
                    kide_core::CompletionKind::Context => Some(CompletionItemKind::MODULE),
                    kide_core::CompletionKind::Keyword => Some(CompletionItemKind::KEYWORD),
                    kide_core::CompletionKind::Type => Some(CompletionItemKind::CLASS),
                    kide_core::CompletionKind::Symbol => Some(CompletionItemKind::FUNCTION),
                };
                CompletionItem {
                    label: c.label,
                    kind,
                    detail: c.detail,
                    ..CompletionItem::default()
                }
            })
            .collect();

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn prepare_rename(
        &self,
        params: tower_lsp::lsp_types::TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;

        let Some(source) = self.source_for_uri(&uri) else {
            return Ok(None);
        };

        let rename_info = kide_core::rename_at(&source, position.line, position.character)
            .ok()
            .flatten();

        let Some((old_name, edits)) = rename_info else {
            return Ok(None);
        };

        if let Some(first) = edits.first() {
            let range = range_from_span(first.span);
            Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range,
                placeholder: old_name,
            }))
        } else {
            Ok(None)
        }
    }

    async fn rename(
        &self,
        params: tower_lsp::lsp_types::RenameParams,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        let Some(source) = self.source_for_uri(&uri) else {
            return Ok(None);
        };

        let rename_info = kide_core::rename_at(&source, position.line, position.character)
            .ok()
            .flatten();

        let Some((_old_name, edits)) = rename_info else {
            return Ok(None);
        };

        let text_edits: Vec<TextEdit> = edits
            .into_iter()
            .map(|edit| TextEdit {
                range: range_from_span(edit.span),
                new_text: new_name.clone(),
            })
            .collect();

        let mut changes = std::collections::HashMap::new();
        changes.insert(uri, text_edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(source) = self.source_for_uri(&uri) else {
            return Ok(None);
        };

        let tokens = kide_core::semantic_tokens(&source).ok().unwrap_or_default();
        if tokens.is_empty() {
            return Ok(None);
        }

        // Encode as LSP delta tokens (each relative to previous)
        let mut lsp_tokens = Vec::with_capacity(tokens.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;

        for token in &tokens {
            let delta_line = token.line - prev_line;
            let delta_start = if delta_line == 0 {
                token.start_char - prev_start
            } else {
                token.start_char
            };

            let token_type = match token.kind {
                kide_core::SemanticTokenKind::Namespace => 0,
                kide_core::SemanticTokenKind::Class => 1,
                kide_core::SemanticTokenKind::Function => 2,
                kide_core::SemanticTokenKind::Event => 3,
                kide_core::SemanticTokenKind::Property => 4,
                kide_core::SemanticTokenKind::Type => 5,
                kide_core::SemanticTokenKind::String => 6,
                kide_core::SemanticTokenKind::Keyword => 7,
            };

            lsp_tokens.push(LspSemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type,
                token_modifiers_bitset: 0,
            });

            prev_line = token.line;
            prev_start = token.start_char;
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: lsp_tokens,
        })))
    }
}

fn diagnostics_for_source(source: &str, uri: &Url) -> Vec<Diagnostic> {
    let base_dir = uri
        .to_file_path()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    match kide_core::check_source_in_dir(source, &base_dir) {
        Ok(report) => {
            let mut diagnostics: Vec<_> = report
                .violations
                .into_iter()
                .map(diagnostic_from_violation)
                .collect();
            attach_binding_dependency_related_information(&mut diagnostics, uri);
            diagnostics
        }
        Err(err) => vec![Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("kide".to_owned()),
            message: err.to_string(),
            ..Diagnostic::default()
        }],
    }
}

fn attach_binding_dependency_related_information(diagnostics: &mut [Diagnostic], uri: &Url) {
    let mut missing_by_path = HashMap::new();
    for diagnostic in diagnostics.iter() {
        if diagnostic_code(diagnostic) != Some(kide_core::CODE_BINDING_FILE_NOT_FOUND) {
            continue;
        }
        let Some(path) = bound_file_path_from_message(&diagnostic.message) else {
            continue;
        };
        missing_by_path.entry(path).or_insert_with(|| {
            (
                diagnostic.range.clone(),
                diagnostic.message.clone(),
                uri.clone(),
            )
        });
    }

    for diagnostic in diagnostics.iter_mut() {
        if diagnostic_code(diagnostic) != Some(kide_core::CODE_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY)
        {
            continue;
        }
        let Some(path) = bound_file_path_from_message(&diagnostic.message) else {
            continue;
        };
        let Some((range, message, missing_uri)) = missing_by_path.get(&path) else {
            continue;
        };
        diagnostic.related_information = Some(vec![DiagnosticRelatedInformation {
            location: Location::new(missing_uri.clone(), range.clone()),
            message: message.clone(),
        }]);
    }
}

fn code_actions_for_diagnostics(uri: &Url, diagnostics: &[Diagnostic]) -> Vec<CodeActionOrCommand> {
    diagnostics
        .iter()
        .flat_map(|diagnostic| {
            [
                preferred_term_code_action(uri, diagnostic),
                missing_symbol_suggestion_code_action(uri, diagnostic),
                remove_bound_symbol_clause_code_action(uri, diagnostic),
                missing_bound_file_code_action(uri, diagnostic),
                remove_duplicate_entry_code_action(uri, diagnostic),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}

fn preferred_term_code_action(uri: &Url, diagnostic: &Diagnostic) -> Option<CodeActionOrCommand> {
    let replacement = preferred_term_from_diagnostic(diagnostic)?;
    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Replace with '{}'", replacement),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range.clone(),
                    new_text: replacement,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }))
}

fn missing_symbol_suggestion_code_action(
    uri: &Url,
    diagnostic: &Diagnostic,
) -> Option<CodeActionOrCommand> {
    let replacement = missing_symbol_suggestion_from_diagnostic(diagnostic)?;
    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Replace with '{}'", replacement),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range.clone(),
                    new_text: replacement,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }))
}

fn missing_bound_file_code_action(
    uri: &Url,
    diagnostic: &Diagnostic,
) -> Option<CodeActionOrCommand> {
    if diagnostic_code(diagnostic) != Some(kide_core::CODE_BINDING_FILE_NOT_FOUND) {
        return None;
    }
    let missing_path = bound_file_path_from_message(&diagnostic.message)?;
    let missing_uri = missing_bound_file_uri(uri, &missing_path)?;

    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Create missing bound file '{}'", missing_path),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Operations(vec![
                DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                    uri: missing_uri,
                    options: Some(CreateFileOptions {
                        overwrite: None,
                        ignore_if_exists: Some(true),
                    }),
                    annotation_id: None,
                })),
            ])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }))
}

fn remove_bound_symbol_clause_code_action(
    uri: &Url,
    diagnostic: &Diagnostic,
) -> Option<CodeActionOrCommand> {
    if diagnostic_code(diagnostic) != Some(kide_core::CODE_COMMAND_BINDING_ARITY_MISMATCH)
        || diagnostic.range.start == diagnostic.range.end
    {
        return None;
    }

    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Remove bound symbol clause".to_owned(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range.clone(),
                    new_text: String::new(),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }))
}

fn remove_duplicate_entry_code_action(
    uri: &Url,
    diagnostic: &Diagnostic,
) -> Option<CodeActionOrCommand> {
    let title = match diagnostic_code(diagnostic) {
        Some(kide_core::CODE_DICTIONARY_DUPLICATE_KEY) => "Remove duplicate dictionary entry",
        Some(kide_core::CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID) => "Remove duplicate forbid entry",
        Some(kide_core::CODE_CONTEXT_BOUNDARY_SELF_FORBID) => "Remove self-forbid entry",
        _ => return None,
    };

    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: title.to_owned(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range.clone(),
                    new_text: String::new(),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }))
}

fn missing_bound_file_uri(uri: &Url, missing_path: &str) -> Option<Url> {
    let missing_path = std::path::PathBuf::from(missing_path);
    let missing_path = if missing_path.is_absolute() {
        missing_path
    } else {
        uri.to_file_path().ok()?.parent()?.join(missing_path)
    };
    Url::from_file_path(missing_path).ok()
}

fn preferred_term_from_diagnostic(diagnostic: &Diagnostic) -> Option<String> {
    if diagnostic_code(diagnostic) != Some(kide_core::CODE_DICTIONARY_TERM_PREFERRED) {
        return None;
    }

    diagnostic
        .data
        .as_ref()
        .and_then(|data| data.get("hint"))
        .and_then(|hint| hint.as_str())
        .and_then(preferred_term_from_hint)
        .or_else(|| preferred_term_from_message(&diagnostic.message))
}

fn preferred_term_from_hint(hint: &str) -> Option<String> {
    quoted_value_after_prefix(hint, "use '")
}

fn preferred_term_from_message(message: &str) -> Option<String> {
    quoted_value_after_prefix(message, "preferred term is '")
}

fn missing_symbol_suggestion_from_diagnostic(diagnostic: &Diagnostic) -> Option<String> {
    if diagnostic_code(diagnostic) != Some(kide_core::CODE_BINDING_SYMBOL_NOT_FOUND) {
        return None;
    }

    diagnostic
        .data
        .as_ref()
        .and_then(|data| data.get("hint"))
        .and_then(|hint| hint.as_str())
        .and_then(missing_symbol_suggestion_from_hint)
}

fn missing_symbol_suggestion_from_hint(hint: &str) -> Option<String> {
    quoted_value_after_prefix(hint, "did you mean '")
}

fn bound_file_path_from_message(message: &str) -> Option<String> {
    quoted_value_after_prefix(message, "bound file '")
}

fn quoted_value_after_prefix(value: &str, prefix: &str) -> Option<String> {
    let start = value.find(prefix)? + prefix.len();
    let remaining = &value[start..];
    let end = remaining.find('\'')?;
    Some(remaining[..end].to_owned())
}

fn diagnostic_code(diagnostic: &Diagnostic) -> Option<&str> {
    match diagnostic.code.as_ref()? {
        NumberOrString::String(code) => Some(code),
        NumberOrString::Number(_) => None,
    }
}

fn diagnostic_from_violation(violation: kide_core::Violation) -> Diagnostic {
    let range = range_for_violation(&violation);
    let severity = match violation.severity {
        kide_core::ViolationSeverity::Error => DiagnosticSeverity::ERROR,
        kide_core::ViolationSeverity::Warning => DiagnosticSeverity::WARNING,
        kide_core::ViolationSeverity::Information => DiagnosticSeverity::INFORMATION,
    };
    let code = violation.code;
    let hint = violation.hint;
    let docs_uri = violation.docs_uri;
    let has_metadata = hint.is_some() || docs_uri.is_some();

    Diagnostic {
        range,
        severity: Some(severity),
        source: Some("kide".to_owned()),
        code: Some(NumberOrString::String(code.to_owned())),
        code_description: docs_uri
            .and_then(|uri| Url::parse(uri).ok())
            .map(|href| CodeDescription { href }),
        data: has_metadata.then(|| {
            serde_json::json!({
                "code": code,
                "hint": hint,
                "docsUri": docs_uri,
            })
        }),
        message: violation.message,
        ..Diagnostic::default()
    }
}

fn range_for_violation(violation: &kide_core::Violation) -> Range {
    let Some(span) = violation.span else {
        return Range::new(Position::new(0, 0), Position::new(0, 1));
    };

    let start_line = span.start_line.saturating_sub(1) as u32;
    let start_column = span.start_column.saturating_sub(1) as u32;
    let end_line = span.end_line.saturating_sub(1) as u32;
    let end_column = span.end_column.saturating_sub(1) as u32;

    Range::new(
        Position::new(start_line, start_column),
        Position::new(end_line, end_column.max(start_column + 1)),
    )
}

fn range_from_span(span: kide_core::ViolationSpan) -> Range {
    let start_line = span.start_line.saturating_sub(1) as u32;
    let start_column = span.start_column.saturating_sub(1) as u32;
    let end_line = span.end_line.saturating_sub(1) as u32;
    let end_column = span.end_column.saturating_sub(1) as u32;

    Range::new(
        Position::new(start_line, start_column),
        Position::new(end_line, end_column.max(start_column + 1)),
    )
}

#[allow(deprecated)]
fn document_symbols_for_source(source: &str) -> Vec<DocumentSymbol> {
    let Ok(program) = kide_parser::parse(source) else {
        return Vec::new();
    };

    let mut symbols = Vec::new();
    for context in program.contexts {
        let mut context_children = Vec::new();
        for element in context.elements {
            let kide_parser::grammar::ContextElement::Aggregate(aggregate) = element else {
                continue;
            };

            let mut aggregate_children = Vec::new();
            for member in aggregate.members {
                match member {
                    kide_parser::grammar::AggregateMember::Command(command) => {
                        aggregate_children.push(DocumentSymbol {
                            name: command.name.text.clone(),
                            detail: Some("command".to_owned()),
                            kind: SymbolKind::METHOD,
                            tags: None,
                            deprecated: None,
                            range: range_from_position(
                                command.name.position.start.line,
                                command.name.position.start.column,
                                command.name.position.end.line,
                                command.name.position.end.column,
                            ),
                            selection_range: range_from_position(
                                command.name.position.start.line,
                                command.name.position.start.column,
                                command.name.position.end.line,
                                command.name.position.end.column,
                            ),
                            children: None,
                        });
                    }
                    kide_parser::grammar::AggregateMember::Invariant(invariant) => {
                        aggregate_children.push(DocumentSymbol {
                            name: invariant.name.text.clone(),
                            detail: Some("invariant".to_owned()),
                            kind: SymbolKind::PROPERTY,
                            tags: None,
                            deprecated: None,
                            range: range_from_position(
                                invariant.name.position.start.line,
                                invariant.name.position.start.column,
                                invariant.name.position.end.line,
                                invariant.name.position.end.column,
                            ),
                            selection_range: range_from_position(
                                invariant.name.position.start.line,
                                invariant.name.position.start.column,
                                invariant.name.position.end.line,
                                invariant.name.position.end.column,
                            ),
                            children: None,
                        });
                    }
                    kide_parser::grammar::AggregateMember::Field(_) => {}
                }
            }

            context_children.push(DocumentSymbol {
                name: aggregate.name.text.clone(),
                detail: Some("aggregate".to_owned()),
                kind: SymbolKind::STRUCT,
                tags: None,
                deprecated: None,
                range: range_from_position(
                    aggregate.name.position.start.line,
                    aggregate.name.position.start.column,
                    aggregate.name.position.end.line,
                    aggregate.name.position.end.column,
                ),
                selection_range: range_from_position(
                    aggregate.name.position.start.line,
                    aggregate.name.position.start.column,
                    aggregate.name.position.end.line,
                    aggregate.name.position.end.column,
                ),
                children: Some(aggregate_children),
            });
        }

        symbols.push(DocumentSymbol {
            name: context.name.text.clone(),
            detail: Some("context".to_owned()),
            kind: SymbolKind::MODULE,
            tags: None,
            deprecated: None,
            range: range_from_position(
                context.name.position.start.line,
                context.name.position.start.column,
                context.name.position.end.line,
                context.name.position.end.column,
            ),
            selection_range: range_from_position(
                context.name.position.start.line,
                context.name.position.start.column,
                context.name.position.end.line,
                context.name.position.end.column,
            ),
            children: Some(context_children),
        });
    }

    symbols
}

fn range_from_position(
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
) -> Range {
    range_from_span(kide_core::ViolationSpan {
        start_line,
        start_column,
        end_line,
        end_column,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        code_actions_for_diagnostics, diagnostic_code, diagnostic_from_violation,
        diagnostics_for_source, document_symbols_for_source, range_for_violation,
    };
    use kide_core::{Violation, ViolationSeverity, ViolationSpan};
    use tower_lsp::lsp_types::{
        CodeActionKind, CodeActionOrCommand, DocumentChangeOperation, DocumentChanges,
        NumberOrString, ResourceOp, Url,
    };

    #[test]
    fn converts_one_based_span_to_lsp_zero_based_range() {
        let violation = Violation {
            severity: ViolationSeverity::Error,
            code: "TEST",
            message: "msg".to_owned(),
            hint: None,
            docs_uri: None,
            span: Some(ViolationSpan {
                start_line: 4,
                start_column: 30,
                end_line: 4,
                end_column: 49,
            }),
        };

        let range = range_for_violation(&violation);
        assert_eq!(range.start.line, 3);
        assert_eq!(range.start.character, 29);
        assert_eq!(range.end.line, 3);
        assert_eq!(range.end.character, 48);
    }

    #[test]
    fn builds_document_outline_symbols() {
        let source = r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "src/domain/order.rs" symbol "Order::ship"
    invariant MustHaveItems bound to "src/domain/order.rs" symbol "Order::verify_not_empty"
  }
}
"#;

        let symbols = document_symbols_for_source(source);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "SalesContext");
        let context_children = symbols[0].children.as_ref().unwrap();
        assert_eq!(context_children.len(), 1);
        assert_eq!(context_children[0].name, "Order");
        let aggregate_children = context_children[0].children.as_ref().unwrap();
        assert_eq!(aggregate_children.len(), 2);
    }

    #[test]
    fn maps_violation_metadata_into_lsp_diagnostic() {
        let violation = Violation {
            severity: ViolationSeverity::Warning,
            code: "TEST_CODE",
            message: "message".to_owned(),
            hint: Some("do something".to_owned()),
            docs_uri: Some("https://docs.kide.dev/diagnostics/test-code"),
            span: Some(ViolationSpan {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            }),
        };

        let diagnostic = diagnostic_from_violation(violation);
        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String("TEST_CODE".to_owned()))
        );
        assert_eq!(
            diagnostic.code_description.unwrap().href.as_str(),
            "https://docs.kide.dev/diagnostics/test-code"
        );
        let data = diagnostic.data.unwrap();
        assert_eq!(data["hint"], "do something");
        assert_eq!(
            data["docsUri"],
            "https://docs.kide.dev/diagnostics/test-code"
        );
        assert_eq!(data["code"], "TEST_CODE");
    }

    #[test]
    fn leaves_lsp_diagnostic_metadata_empty_when_violation_has_none() {
        let violation = Violation {
            severity: ViolationSeverity::Warning,
            code: "TEST_CODE",
            message: "message".to_owned(),
            hint: None,
            docs_uri: None,
            span: Some(ViolationSpan {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            }),
        };

        let diagnostic = diagnostic_from_violation(violation);
        assert!(diagnostic.code_description.is_none());
        assert!(diagnostic.data.is_none());
    }

    #[test]
    fn attaches_related_information_to_unverified_dependency_diagnostics() {
        let uri = Url::from_file_path(
            std::env::temp_dir().join(format!("kide-lsp-test-{}.kide", std::process::id())),
        )
        .unwrap();
        let source = r#"
context SalesContext {
  aggregate Order {
    command ship() bound to "missing.rs" symbol "Order::ship"
  }
}
"#;

        let diagnostics = diagnostics_for_source(source, &uri);
        let missing = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic_code(diagnostic) == Some(kide_core::CODE_BINDING_FILE_NOT_FOUND)
            })
            .unwrap();
        let unverified = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic_code(diagnostic)
                    == Some(kide_core::CODE_BINDING_SYMBOL_UNVERIFIED_DEPENDENCY)
            })
            .unwrap();
        let related = unverified.related_information.as_ref().unwrap();

        assert_eq!(related.len(), 1);
        assert_eq!(related[0].location.uri, uri);
        assert_eq!(related[0].location.range, missing.range);
        assert_eq!(related[0].message, missing.message);
    }

    #[test]
    fn builds_quick_fix_actions_for_preferred_dictionary_terms() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostic = diagnostic_from_violation(Violation {
            severity: ViolationSeverity::Warning,
            code: kide_core::CODE_DICTIONARY_TERM_PREFERRED,
            message: "dictionary term 'Term' appears in 'x' but preferred term is 'Preferred'"
                .to_owned(),
            hint: Some("use 'Preferred' instead of 'Term'".to_owned()),
            docs_uri: None,
            span: Some(ViolationSpan {
                start_line: 2,
                start_column: 3,
                end_line: 2,
                end_column: 7,
            }),
        });

        let actions = code_actions_for_diagnostics(&uri, &[diagnostic.clone()]);
        assert_eq!(actions.len(), 1);
        let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
            panic!("expected code action");
        };
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let edits = action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.get(&uri))
            .unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range, diagnostic.range);
        assert_eq!(edits[0].new_text, "Preferred");
    }

    #[test]
    fn builds_quick_fix_actions_for_missing_binding_files() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let missing_file = "/tmp/missing-bound-file.rs";
        let diagnostic = diagnostic_from_violation(Violation {
            severity: ViolationSeverity::Error,
            code: kide_core::CODE_BINDING_FILE_NOT_FOUND,
            message: format!("bound file '{}' does not exist", missing_file),
            hint: None,
            docs_uri: None,
            span: Some(ViolationSpan {
                start_line: 2,
                start_column: 3,
                end_line: 2,
                end_column: 7,
            }),
        });

        let actions = code_actions_for_diagnostics(&uri, &[diagnostic]);
        assert_eq!(actions.len(), 1);
        let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
            panic!("expected code action");
        };
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let edit = action.edit.as_ref().unwrap();
        let Some(DocumentChanges::Operations(operations)) = edit.document_changes.as_ref() else {
            panic!("expected document changes operations");
        };
        assert_eq!(operations.len(), 1);
        let DocumentChangeOperation::Op(ResourceOp::Create(create_file)) = &operations[0] else {
            panic!("expected create file operation");
        };
        assert_eq!(create_file.uri, Url::from_file_path(missing_file).unwrap());
    }

    #[test]
    fn builds_quick_fix_actions_for_missing_symbol_suggestions() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostic = diagnostic_from_violation(Violation {
            severity: ViolationSeverity::Error,
            code: kide_core::CODE_BINDING_SYMBOL_NOT_FOUND,
            message: "binding symbol 'Order::shipp' was not found".to_owned(),
            hint: Some("did you mean 'Order::ship'?".to_owned()),
            docs_uri: None,
            span: Some(ViolationSpan {
                start_line: 2,
                start_column: 3,
                end_line: 2,
                end_column: 14,
            }),
        });

        let actions = code_actions_for_diagnostics(&uri, &[diagnostic.clone()]);
        assert_eq!(actions.len(), 1);
        let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
            panic!("expected code action");
        };
        let edits = action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.get(&uri))
            .unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range, diagnostic.range);
        assert_eq!(edits[0].new_text, "Order::ship");
    }

    #[test]
    fn skips_missing_symbol_suggestion_quick_fix_when_hint_is_absent() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostic = diagnostic_from_violation(Violation {
            severity: ViolationSeverity::Error,
            code: kide_core::CODE_BINDING_SYMBOL_NOT_FOUND,
            message: "binding symbol 'Order::shipp' was not found".to_owned(),
            hint: None,
            docs_uri: None,
            span: Some(ViolationSpan {
                start_line: 2,
                start_column: 3,
                end_line: 2,
                end_column: 14,
            }),
        });

        let actions = code_actions_for_diagnostics(&uri, &[diagnostic]);
        assert!(actions.is_empty());
    }

    #[test]
    fn builds_quick_fix_actions_for_command_binding_arity_mismatch() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostic = diagnostic_from_violation(Violation {
            severity: ViolationSeverity::Error,
            code: kide_core::CODE_COMMAND_BINDING_ARITY_MISMATCH,
            message: "command arity mismatch".to_owned(),
            hint: Some("adjust command parameters".to_owned()),
            docs_uri: None,
            span: Some(ViolationSpan {
                start_line: 2,
                start_column: 20,
                end_line: 2,
                end_column: 34,
            }),
        });

        let actions = code_actions_for_diagnostics(&uri, &[diagnostic.clone()]);
        assert_eq!(actions.len(), 1);
        let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
            panic!("expected code action");
        };
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert_eq!(action.title, "Remove bound symbol clause");
        let edits = action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.get(&uri))
            .unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range, diagnostic.range);
        assert_eq!(edits[0].new_text, "");
    }

    #[test]
    fn preserves_existing_quick_fixes_when_arity_mismatch_fix_exists() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostics = vec![
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_COMMAND_BINDING_ARITY_MISMATCH,
                message: "command arity mismatch".to_owned(),
                hint: Some("adjust command parameters".to_owned()),
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 1,
                    start_column: 10,
                    end_line: 1,
                    end_column: 25,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Warning,
                code: kide_core::CODE_DICTIONARY_TERM_PREFERRED,
                message: "dictionary term 'Term' appears in 'x' but preferred term is 'Preferred'"
                    .to_owned(),
                hint: Some("use 'Preferred' instead of 'Term'".to_owned()),
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 2,
                    start_column: 3,
                    end_line: 2,
                    end_column: 7,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_BINDING_SYMBOL_NOT_FOUND,
                message: "binding symbol 'Order::shipp' was not found".to_owned(),
                hint: Some("did you mean 'Order::ship'?".to_owned()),
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 3,
                    start_column: 1,
                    end_line: 3,
                    end_column: 13,
                }),
            }),
        ];

        let actions = code_actions_for_diagnostics(&uri, &diagnostics);
        assert_eq!(actions.len(), 3);
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Remove bound symbol clause"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Replace with 'Preferred'"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Replace with 'Order::ship'"
        }));
    }

    #[test]
    fn builds_quick_fix_actions_for_duplicate_and_boundary_entries() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostics = vec![
            (
                kide_core::CODE_DICTIONARY_DUPLICATE_KEY,
                "Remove duplicate dictionary entry",
                1,
            ),
            (
                kide_core::CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID,
                "Remove duplicate forbid entry",
                2,
            ),
            (
                kide_core::CODE_CONTEXT_BOUNDARY_SELF_FORBID,
                "Remove self-forbid entry",
                3,
            ),
        ]
        .into_iter()
        .map(|(code, _title, line)| {
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code,
                message: "duplicate entry".to_owned(),
                hint: None,
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: line,
                    start_column: 1,
                    end_line: line,
                    end_column: 10,
                }),
            })
        })
        .collect::<Vec<_>>();

        let actions = code_actions_for_diagnostics(&uri, &diagnostics);
        assert_eq!(actions.len(), 3);
        for (action, diagnostic) in actions.iter().zip(diagnostics.iter()) {
            let CodeActionOrCommand::CodeAction(action) = action else {
                panic!("expected code action");
            };
            let edits = action
                .edit
                .as_ref()
                .and_then(|edit| edit.changes.as_ref())
                .and_then(|changes| changes.get(&uri))
                .unwrap();
            assert_eq!(edits.len(), 1);
            assert_eq!(edits[0].range, diagnostic.range);
            assert_eq!(edits[0].new_text, "");
        }
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Remove duplicate dictionary entry"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Remove duplicate forbid entry"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Remove self-forbid entry"
        }));
    }

    #[test]
    fn preserves_preferred_term_quick_fix_when_missing_file_fix_exists() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostics = vec![
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_BINDING_FILE_NOT_FOUND,
                message: "bound file '/tmp/missing-bound-file.rs' does not exist".to_owned(),
                hint: None,
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 2,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Warning,
                code: kide_core::CODE_DICTIONARY_TERM_PREFERRED,
                message: "dictionary term 'Term' appears in 'x' but preferred term is 'Preferred'"
                    .to_owned(),
                hint: Some("use 'Preferred' instead of 'Term'".to_owned()),
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 2,
                    start_column: 3,
                    end_line: 2,
                    end_column: 7,
                }),
            }),
        ];

        let actions = code_actions_for_diagnostics(&uri, &diagnostics);
        assert_eq!(actions.len(), 2);
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Replace with 'Preferred'"
        }));
    }

    #[test]
    fn preserves_existing_quick_fixes_when_duplicate_entry_fixes_exist() {
        let uri = Url::parse("file:///tmp/domain.kide").unwrap();
        let diagnostics = vec![
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_BINDING_FILE_NOT_FOUND,
                message: "bound file '/tmp/missing-bound-file.rs' does not exist".to_owned(),
                hint: None,
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 2,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Warning,
                code: kide_core::CODE_DICTIONARY_TERM_PREFERRED,
                message: "dictionary term 'Term' appears in 'x' but preferred term is 'Preferred'"
                    .to_owned(),
                hint: Some("use 'Preferred' instead of 'Term'".to_owned()),
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 2,
                    start_column: 3,
                    end_line: 2,
                    end_column: 7,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_BINDING_SYMBOL_NOT_FOUND,
                message: "binding symbol 'Order::shipp' was not found".to_owned(),
                hint: Some("did you mean 'Order::ship'?".to_owned()),
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 3,
                    start_column: 1,
                    end_line: 3,
                    end_column: 13,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_DICTIONARY_DUPLICATE_KEY,
                message: "duplicate entry".to_owned(),
                hint: None,
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 4,
                    start_column: 1,
                    end_line: 4,
                    end_column: 4,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_CONTEXT_BOUNDARY_DUPLICATE_FORBID,
                message: "duplicate entry".to_owned(),
                hint: None,
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 5,
                    start_column: 1,
                    end_line: 5,
                    end_column: 4,
                }),
            }),
            diagnostic_from_violation(Violation {
                severity: ViolationSeverity::Error,
                code: kide_core::CODE_CONTEXT_BOUNDARY_SELF_FORBID,
                message: "duplicate entry".to_owned(),
                hint: None,
                docs_uri: None,
                span: Some(ViolationSpan {
                    start_line: 6,
                    start_column: 1,
                    end_line: 6,
                    end_column: 4,
                }),
            }),
        ];

        let actions = code_actions_for_diagnostics(&uri, &diagnostics);
        assert_eq!(actions.len(), 6);
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Replace with 'Preferred'"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Create missing bound file '/tmp/missing-bound-file.rs'"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Replace with 'Order::ship'"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Remove duplicate dictionary entry"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Remove duplicate forbid entry"
        }));
        assert!(actions.iter().any(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return false;
            };
            action.title == "Remove self-forbid entry"
        }));
    }
}
