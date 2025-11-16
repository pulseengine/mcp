#![allow(warnings)]
use exports::wasi::mcp::handlers::Guest;

wit_bindgen::generate!({
    world: "hello-mcp",
    path: "wit",
});

struct Component;

impl Guest for Component {
    fn call_tool(name: String, arguments: Vec<u8>) -> Result<exports::wasi::mcp::handlers::CallToolResult, wasi::mcp::types::Error> {
        eprintln!("[COMPONENT] call_tool: name={}", name);
        
        // Simple echo tool
        if name == "echo" {
            // Parse arguments JSON
            let args_str = String::from_utf8(arguments.clone())
                .unwrap_or_else(|_| "{}".to_string());
            
            eprintln!("[COMPONENT] echo tool called with: {}", args_str);
            
            // Create text content response
            let content = vec![
                wasi::mcp::content::ContentBlock::Text(wasi::mcp::content::TextContent {
                    text: format!("Echo: {}", args_str),
                    annotations: None,
                })
            ];
            
            Ok(exports::wasi::mcp::handlers::CallToolResult {
                content,
                is_error: Some(false),
            })
        } else {
            eprintln!("[COMPONENT] Unknown tool: {}", name);
            Err(wasi::mcp::types::Error::ToolNotFound)
        }
    }

    fn read_resource(uri: String) -> Result<exports::wasi::mcp::handlers::ReadResourceResult, wasi::mcp::types::Error> {
        eprintln!("[COMPONENT] read_resource: uri={}", uri);
        Err(wasi::mcp::types::Error::ResourceNotFound)
    }

    fn get_prompt(name: String, arguments: Option<Vec<exports::wasi::mcp::handlers::PromptArgument>>) -> Result<exports::wasi::mcp::handlers::GetPromptResult, wasi::mcp::types::Error> {
        eprintln!("[COMPONENT] get_prompt: name={}", name);
        Err(wasi::mcp::types::Error::PromptNotFound)
    }
}

export!(Component);
