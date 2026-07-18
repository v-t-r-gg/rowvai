use rowva_core::{
    ActorContext, Command, FieldKind, OperationRequest, RowvaApplication, TextConfig,
};
use rowva_store_sqlite::SqliteApplication;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command as ProcessCommand, Stdio};

fn response(reader: &mut BufReader<std::process::ChildStdout>) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    serde_json::from_str(&line)
        .unwrap_or_else(|error| panic!("stdout was not MCP JSON: {error}: {line}"))
}

#[test]
fn stdio_initializes_lists_tools_and_calls_workspace_description() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("smoke.rowva");
    let mut app = SqliteApplication::create(&workspace, "Smoke CRM").unwrap();
    app.execute(OperationRequest::new(
        ActorContext::local_user(),
        Command::CreateObject {
            display_name: "Contacts".into(),
            key: None,
        },
    ))
    .unwrap();
    let object = app.list_objects().unwrap().remove(0).id;
    app.execute(OperationRequest::new(
        ActorContext::local_user(),
        Command::CreateField {
            object_id: object.clone(),
            display_name: "Name".into(),
            key: None,
            kind: FieldKind::Text(TextConfig::default()),
            required: false,
            unique: false,
        },
    ))
    .unwrap();
    let field = app.list_fields(&object).unwrap().remove(0).id;
    drop(app);

    let mut child = ProcessCommand::new(env!("CARGO_BIN_EXE_rowva-mcp"))
        .args([
            "serve",
            "--workspace",
            workspace.to_str().unwrap(),
            "--actor-id",
            "smoke_agent",
            "--actor-name",
            "Smoke Agent",
            "--actor-version",
            "1.0",
            "--capability",
            "workspace.read",
            "--capability",
            "schema.read",
            "--capability",
            "records.read",
            "--capability",
            "records.create",
            "--capability",
            "records.update",
            "--capability",
            "operations.read",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    writeln!(stdin,"{}",json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"rowva-smoke","version":"1"}}})).unwrap();
    stdin.flush().unwrap();
    let initialized = response(&mut stdout);
    assert_eq!(initialized["id"], 1);
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}})
    )
    .unwrap();
    stdin.flush().unwrap();
    let tools = response(&mut stdout);
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 8);
    writeln!(stdin,"{}",json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rowva_workspace_describe","arguments":{}}})).unwrap();
    stdin.flush().unwrap();
    let described = response(&mut stdout);
    assert_eq!(
        described["result"]["structuredContent"]["workspace_display_name"],
        "Smoke CRM"
    );
    writeln!(stdin,"{}",json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"rowva_operation_preview","arguments":{"command":{"type":"create_record","object_id":object,"values":{field.as_str():"Ada"}},"reason":"stdio smoke","idempotency_key":"smoke-create-1"}}})).unwrap();
    stdin.flush().unwrap();
    let preview = response(&mut stdout);
    let proposal = &preview["result"]["structuredContent"];
    assert_eq!(proposal["status"], "awaiting_approval");
    assert!(proposal["approval_request_id"].is_string());
    let operation = proposal["operation_id"].as_str().unwrap();
    let fingerprint = proposal["preview_fingerprint"].as_str().unwrap();
    writeln!(stdin,"{}",json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"rowva_operation_commit","arguments":{"operation_id":operation,"preview_fingerprint":fingerprint}}})).unwrap();
    stdin.flush().unwrap();
    let commit = response(&mut stdout);
    assert_eq!(
        commit["result"]["structuredContent"]["error"]["code"],
        "approval_required"
    );
    writeln!(stdin,"{}",json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"rowva_operation_get","arguments":{"operation_id":operation}}})).unwrap();
    stdin.flush().unwrap();
    let inspected = response(&mut stdout);
    assert_eq!(
        inspected["result"]["structuredContent"]["status"],
        "awaiting_approval"
    );
    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.contains("mcp.server.start"));
}
