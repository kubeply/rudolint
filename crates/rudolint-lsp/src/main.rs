use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use lsp_types::{
    CodeActionOrCommand, CodeActionProviderCapability, CodeActionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    HoverProviderCapability, InitializeParams, InitializeResult, Position,
    PublishDiagnosticsParams, ServerCapabilities, ServerInfo, TextDocumentItem,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WorkspaceFolder,
};
use rudolint_lsp::DocumentLinter;
use rudolint_rules::Profile;

fn main() -> Result<()> {
    let (connection, io_threads) = Connection::stdio();
    let mut server = Server::initialize(&connection)?;
    server.run(&connection)?;
    drop(connection);
    io_threads.join()?;
    Ok(())
}

#[derive(Debug)]
struct Server {
    documents: HashMap<String, TextDocumentItem>,
    workspace_linter: DocumentLinter,
    shutdown_requested: bool,
}

impl Server {
    fn initialize(connection: &Connection) -> Result<Self> {
        let (request_id, initialize_params) = connection.initialize_start()?;
        let params = serde_json::from_value::<InitializeParams>(initialize_params)
            .context("failed to parse initialize params")?;
        let workspace_folders = workspace_folders(&params);
        let workspace_linter =
            DocumentLinter::discover_for_workspace(Profile::Default, workspace_folders.as_slice())
                .unwrap_or_else(|error| {
                    eprintln!("rudolint-lsp: failed to discover workspace config: {error}");
                    DocumentLinter::default()
                });

        let result = InitializeResult {
            capabilities: capabilities(),
            server_info: Some(ServerInfo {
                name: "rudolint-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        };
        connection.initialize_finish(request_id, serde_json::to_value(result)?)?;

        Ok(Self {
            documents: HashMap::new(),
            workspace_linter,
            shutdown_requested: false,
        })
    }

    fn run(&mut self, connection: &Connection) -> Result<()> {
        for message in &connection.receiver {
            match message {
                Message::Request(request) => {
                    self.handle_request(connection, request)?;
                }
                Message::Notification(notification) => {
                    if self.handle_notification(connection, notification)? {
                        return Ok(());
                    }
                }
                Message::Response(_) => {}
            }
        }

        Ok(())
    }

    fn handle_request(&mut self, connection: &Connection, request: Request) -> Result<()> {
        match request.method.as_str() {
            lsp_types::request::Shutdown::METHOD => {
                self.shutdown_requested = true;
                send_response(
                    connection,
                    Response::new_ok(request.id, serde_json::Value::Null),
                )
            }
            lsp_types::request::HoverRequest::METHOD => {
                let id = request.id;
                let params = match serde_json::from_value(request.params) {
                    Ok(params) => params,
                    Err(error) => {
                        return send_response(
                            connection,
                            Response::new_err(
                                id,
                                ErrorCode::InvalidParams as i32,
                                error.to_string(),
                            ),
                        );
                    }
                };
                let hover = self.hover(params);
                send_response(
                    connection,
                    Response::new_ok(id, serde_json::to_value(hover)?),
                )
            }
            lsp_types::request::CodeActionRequest::METHOD => {
                let id = request.id;
                let params = match serde_json::from_value(request.params) {
                    Ok(params) => params,
                    Err(error) => {
                        return send_response(
                            connection,
                            Response::new_err(
                                id,
                                ErrorCode::InvalidParams as i32,
                                error.to_string(),
                            ),
                        );
                    }
                };
                match self.code_actions(params) {
                    Ok(actions) => send_response(
                        connection,
                        Response::new_ok(id, serde_json::to_value(actions)?),
                    ),
                    Err(error) => send_response(
                        connection,
                        Response::new_err(id, ErrorCode::InternalError as i32, error.to_string()),
                    ),
                }
            }
            _ => send_response(
                connection,
                Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    format!("unsupported request: {}", request.method),
                ),
            ),
        }
    }

    fn handle_notification(
        &mut self,
        connection: &Connection,
        notification: Notification,
    ) -> Result<bool> {
        match notification.method.as_str() {
            lsp_types::notification::Exit::METHOD => {
                if self.shutdown_requested {
                    return Ok(true);
                }

                std::process::exit(1);
            }
            lsp_types::notification::DidOpenTextDocument::METHOD => {
                let params = match serde_json::from_value(notification.params) {
                    Ok(params) => params,
                    Err(error) => {
                        eprintln!(
                            "rudolint-lsp: failed to parse textDocument/didOpen params: {error}"
                        );
                        return Ok(false);
                    }
                };
                if let Err(error) = self.did_open(connection, params) {
                    eprintln!("rudolint-lsp: failed to handle textDocument/didOpen: {error}");
                }
                Ok(false)
            }
            lsp_types::notification::DidChangeTextDocument::METHOD => {
                let params = match serde_json::from_value(notification.params) {
                    Ok(params) => params,
                    Err(error) => {
                        eprintln!(
                            "rudolint-lsp: failed to parse textDocument/didChange params: {error}"
                        );
                        return Ok(false);
                    }
                };
                if let Err(error) = self.did_change(connection, params) {
                    eprintln!("rudolint-lsp: failed to handle textDocument/didChange: {error}");
                }
                Ok(false)
            }
            lsp_types::notification::DidCloseTextDocument::METHOD => {
                let params = match serde_json::from_value(notification.params) {
                    Ok(params) => params,
                    Err(error) => {
                        eprintln!(
                            "rudolint-lsp: failed to parse textDocument/didClose params: {error}"
                        );
                        return Ok(false);
                    }
                };
                if let Err(error) = self.did_close(connection, params) {
                    eprintln!("rudolint-lsp: failed to handle textDocument/didClose: {error}");
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn did_open(
        &mut self,
        connection: &Connection,
        params: DidOpenTextDocumentParams,
    ) -> Result<()> {
        let uri = params.text_document.uri.clone();
        self.documents
            .insert(uri.as_str().to_string(), params.text_document);
        self.publish_diagnostics(connection, &uri)
    }

    fn did_change(
        &mut self,
        connection: &Connection,
        params: DidChangeTextDocumentParams,
    ) -> Result<()> {
        let Some(text) = full_document_text(&params)?.map(str::to_string) else {
            return Ok(());
        };

        let uri = params.text_document.uri;
        let key = uri.as_str().to_string();
        let document = self.documents.entry(key).or_insert_with(|| {
            TextDocumentItem::new(
                uri.clone(),
                "dockerfile".to_string(),
                params.text_document.version,
                String::new(),
            )
        });
        document.version = params.text_document.version;
        document.text = text;

        self.publish_diagnostics(connection, &uri)
    }

    fn did_close(
        &mut self,
        connection: &Connection,
        params: DidCloseTextDocumentParams,
    ) -> Result<()> {
        self.documents.remove(params.text_document.uri.as_str());
        publish(connection, params.text_document.uri, Vec::new(), None)
    }

    fn hover(&self, params: lsp_types::HoverParams) -> Option<lsp_types::Hover> {
        let uri = params.text_document_position_params.text_document.uri;
        let document = self.documents.get(uri.as_str())?;
        let code = rule_code_at_position(
            document.text.as_str(),
            params.text_document_position_params.position,
        )?;

        self.linter_for_uri(&uri).hover_for_rule(&code)
    }

    fn code_actions(&self, params: lsp_types::CodeActionParams) -> Result<CodeActionResponse> {
        let uri = params.text_document.uri;
        let Some(document) = self.documents.get(uri.as_str()) else {
            return Ok(Vec::new());
        };

        Ok(self
            .linter_for_uri(&uri)
            .code_actions_for_document(document)?
            .into_iter()
            .map(CodeActionOrCommand::CodeAction)
            .collect())
    }

    fn publish_diagnostics(&self, connection: &Connection, uri: &Uri) -> Result<()> {
        let Some(document) = self.documents.get(uri.as_str()) else {
            return publish(connection, uri.clone(), Vec::new(), None);
        };

        let diagnostics = self
            .linter_for_uri(uri)
            .lint_open_document(document)
            .unwrap_or_else(|error| {
                eprintln!("rudolint-lsp: failed to lint {}: {error}", uri.as_str());
                Vec::new()
            });

        publish(connection, uri.clone(), diagnostics, Some(document.version))
    }

    fn linter_for_uri(&self, uri: &Uri) -> DocumentLinter {
        DocumentLinter::discover_for_document(Profile::Default, uri).unwrap_or_else(|error| {
            eprintln!(
                "rudolint-lsp: failed to discover config for {}: {error}",
                uri.as_str()
            );
            self.workspace_linter.clone()
        })
    }
}

fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..ServerCapabilities::default()
    }
}

fn workspace_folders(params: &InitializeParams) -> Vec<WorkspaceFolder> {
    if let Some(folders) = params.workspace_folders.clone() {
        return folders;
    }

    #[expect(
        deprecated,
        reason = "root_uri is the best fallback for older clients without workspaceFolders"
    )]
    params
        .root_uri
        .clone()
        .map(|uri| WorkspaceFolder {
            uri,
            name: "workspace".to_string(),
        })
        .into_iter()
        .collect()
}

fn full_document_text(params: &DidChangeTextDocumentParams) -> Result<Option<&str>> {
    if params
        .content_changes
        .iter()
        .any(|change| change.range.is_some() || change.range_length.is_some())
    {
        bail!(
            "incremental textDocument/didChange is not supported yet for {}",
            params.text_document.uri.as_str()
        );
    }

    Ok(params
        .content_changes
        .last()
        .map(|change| change.text.as_str()))
}

fn publish(
    connection: &Connection,
    uri: Uri,
    diagnostics: Vec<lsp_types::Diagnostic>,
    version: Option<i32>,
) -> Result<()> {
    send_notification(
        connection,
        Notification::new(
            lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
            PublishDiagnosticsParams {
                uri,
                diagnostics,
                version,
            },
        ),
    )
}

fn send_notification(connection: &Connection, notification: Notification) -> Result<()> {
    connection
        .sender
        .send(Message::Notification(notification))?;
    Ok(())
}

fn send_response(connection: &Connection, response: Response) -> Result<()> {
    connection.sender.send(Message::Response(response))?;
    Ok(())
}

fn rule_code_at_position(text: &str, position: Position) -> Option<String> {
    let line = text.lines().nth(position.line as usize)?;
    let character = position.character as usize;
    let byte_index = byte_index_for_character(line, character);
    let token = token_at_byte(line, byte_index)?;

    is_rule_code(token).then(|| token.to_ascii_uppercase())
}

fn byte_index_for_character(line: &str, character: usize) -> usize {
    line.char_indices()
        .nth(character)
        .map(|(index, _)| index)
        .unwrap_or(line.len())
}

fn token_at_byte(line: &str, byte_index: usize) -> Option<&str> {
    let start = line[..byte_index]
        .rfind(|character: char| !character.is_ascii_alphanumeric())
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = line[byte_index..]
        .find(|character: char| !character.is_ascii_alphanumeric())
        .map(|index| byte_index + index)
        .unwrap_or(line.len());

    (start < end).then(|| &line[start..end])
}

fn is_rule_code(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    let Some(prefix) = upper.get(..3) else {
        return false;
    };

    matches!(prefix, "RDL" | "RDK" | "RSC")
        && upper.len() == 7
        && upper[3..]
            .chars()
            .all(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use lsp_types::Position;

    use super::{rule_code_at_position, token_at_byte};

    #[test]
    fn finds_rule_code_at_cursor_position() {
        let text = "# hadolint ignore=RDL3007\nFROM alpine:latest\n";

        assert_eq!(
            rule_code_at_position(text, Position::new(0, 20)).as_deref(),
            Some("RDL3007")
        );
    }

    #[test]
    fn normalizes_lowercase_rule_code_at_cursor_position() {
        let text = "# rudolint ignore=rdk1004\nRUN echo ok\n";

        assert_eq!(
            rule_code_at_position(text, Position::new(0, 20)).as_deref(),
            Some("RDK1004")
        );
    }

    #[test]
    fn skips_unknown_token_at_cursor_position() {
        let text = "FROM alpine:latest\n";

        assert_eq!(rule_code_at_position(text, Position::new(0, 5)), None);
    }

    #[test]
    fn token_lookup_handles_cursor_at_end_of_token() {
        assert_eq!(token_at_byte("ignore=RDL3007", 14), Some("RDL3007"));
    }
}
