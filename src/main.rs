// src/main.rs - Fixed version with proper temporary file handling
use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use serde::{Deserialize, Serialize};
use actix_cors::Cors;
use std::collections::HashMap;
use sqlx::PgPool;
use dotenv::dotenv;
use std::env;
use std::process::{Command, Stdio};
use tokio::time::{timeout, Duration};
use std::fs;

mod lexer;
mod parser;
mod evaluator;
mod object;

#[derive(Debug, sqlx::Type, Clone)]
#[sqlx(type_name = "ExecutionStatus", rename_all = "SCREAMING_SNAKE_CASE")]
enum ExecutionStatus {
    Pending,
    Success,
    Error,
    Timeout,
    MemoryLimit,
}

#[derive(Deserialize)]
struct CompileRequest {
    code: String,
    language: String, // "rust", "python", "c", "cpp", "javascript", "go", or "custom"
}

#[derive(Serialize)]
struct CompileResponse {
    result: Option<String>,
    error: Option<String>,
    execution_time_ms: Option<u64>,
}

// Enhanced security for code execution
const EXECUTION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_OUTPUT_SIZE: usize = 10_000; // 10KB max output

async fn compile_handler(req: web::Json<CompileRequest>, pool: web::Data<PgPool>) -> impl Responder {
    let start_time = std::time::Instant::now();
    let code = &req.code;
    let language = &req.language.to_lowercase();

    // Security: Basic input validation
    if code.len() > 50_000 {
        return HttpResponse::BadRequest().json(CompileResponse {
            result: None,
            error: Some("Code too large (max 50KB)".to_string()),
            execution_time_ms: Some(start_time.elapsed().as_millis() as u64),
        });
    }

    let result = match language.as_str() {
        "custom" => execute_custom_language(code).await,
        "rust" => execute_rust_code(code).await,
        "python" => execute_python_code(code).await,
        "c" => execute_c_code(code).await,
        "cpp" | "c++" => execute_cpp_code(code).await,
        "javascript" | "js" => execute_javascript_code(code).await,
        "go" => execute_go_code(code).await,
        _ => Err("Unsupported language. Use: custom, rust, python, c, cpp, javascript, or go".to_string()),
    };

    let execution_time = start_time.elapsed().as_millis() as u64;

    let response = match result {
        Ok(output) => {
            // Log success to database - using Prisma column names
            let _ = sqlx::query!(
                r#"INSERT INTO executions (code, result, status, execution_time_ms, language) VALUES ($1, $2, $3, $4, $5)"#,
                code,
                Some(output.clone()),
                ExecutionStatus::Success as _,
                execution_time as i32,
                language
            )
            .execute(pool.get_ref())
            .await;

            CompileResponse {
                result: Some(output),
                error: None,
                execution_time_ms: Some(execution_time),
            }
        }
        Err(error) => {
            // Log error to database - using Prisma column names
            let _ = sqlx::query!(
                r#"INSERT INTO executions (code, error, status, execution_time_ms, language) VALUES ($1, $2, $3, $4, $5)"#,
                code,
                Some(error.clone()),
                ExecutionStatus::Error as _,
                execution_time as i32,
                language
            )
            .execute(pool.get_ref())
            .await;

            CompileResponse {
                result: None,
                error: Some(error),
                execution_time_ms: Some(execution_time),
            }
        }
    };

    HttpResponse::Ok().json(response)
}

// Execute custom language (your interpreter)
async fn execute_custom_language(code: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(code)?;
    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse_program()?;
    let mut env = HashMap::new();
    
    // Execute the code
    let result = evaluator::evaluate(&ast, &mut env)?;
    
    // Get any output from print statements
    let output = evaluator::get_output();
    
    // Combine print output with result
    if !output.is_empty() {
        // If there was print output, return that
        Ok(output.trim_end().to_string())
    } else {
        // If no print output, return the final result value
        match result {
            object::Object::Null => Ok(String::new()),
            object::Object::String(s) if s.is_empty() => Ok(String::new()),
            other => Ok(other.to_string())
        }
    }
}

// Fixed Rust code execution
async fn execute_rust_code(code: &str) -> Result<String, String> {
    // Create temporary directory
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    // Create rust file with proper name
    let rust_file = temp_dir.path().join("main.rs");
    
    // Wrap code in main function if not present
    let wrapped_code = if !code.contains("fn main") {
        format!("fn main() {{\n{}\n}}", code)
    } else {
        code.to_string()
    };
    
    fs::write(&rust_file, wrapped_code)
        .map_err(|e| format!("Failed to write code: {}", e))?;
    
    // Use proper executable extension for Windows
    let exe_file = if cfg!(target_os = "windows") {
        temp_dir.path().join("main.exe")
    } else {
        temp_dir.path().join("main")
    };
    
    // Compile Rust code
    let compile_output = timeout(EXECUTION_TIMEOUT, async {
        Command::new("rustc")
            .arg(&rust_file)
            .arg("-o")
            .arg(&exe_file)
            .arg("--edition=2021")
            .arg("-A")
            .arg("warnings")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Compilation timeout".to_string())?
    .map_err(|e| format!("Compilation failed: {}", e))?;
    
    if !compile_output.status.success() {
        return Err(format!("Compilation error: {}", 
            String::from_utf8_lossy(&compile_output.stderr)));
    }
    
    // Execute compiled binary
    let run_output = timeout(EXECUTION_TIMEOUT, async {
        Command::new(&exe_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Execution timeout".to_string())?
    .map_err(|e| format!("Execution failed: {}", e))?;
    
    if !run_output.status.success() {
        return Err(format!("Runtime error: {}", 
            String::from_utf8_lossy(&run_output.stderr)));
    }
    
    let output = String::from_utf8_lossy(&run_output.stdout);
    if output.len() > MAX_OUTPUT_SIZE {
        return Err("Output too large".to_string());
    }
    
    Ok(output.to_string())
}

// Execute Python code with multiple fallbacks
async fn execute_python_code(code: &str) -> Result<String, String> {
    // Try different Python commands in order
    let python_commands = ["python", "python3", "py"];
    
    for cmd in &python_commands {
        // Check if this Python command is available
        let python_check = Command::new(cmd)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
            
        if python_check.is_ok() {
            let output = timeout(EXECUTION_TIMEOUT, async {
                Command::new(cmd)
                    .arg("-c")
                    .arg(code)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
            })
            .await
            .map_err(|_| "Execution timeout".to_string())?
            .map_err(|e| format!("Failed to execute Python: {}", e))?;
            
            if !output.status.success() {
                return Err(format!("Python error: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
            
            let result = String::from_utf8_lossy(&output.stdout);
            if result.len() > MAX_OUTPUT_SIZE {
                return Err("Output too large".to_string());
            }
            
            return Ok(result.to_string());
        }
    }
    
    Err("Python is not installed or not accessible. Please install Python or use a different language.".to_string())
}

// Fixed C code execution
async fn execute_c_code(code: &str) -> Result<String, String> {
    // Create temporary directory
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    // Create C file with proper name
    let c_file = temp_dir.path().join("main.c");
    
    fs::write(&c_file, code)
        .map_err(|e| format!("Failed to write code: {}", e))?;
    
    // Use proper executable extension for Windows
    let exe_file = if cfg!(target_os = "windows") {
        temp_dir.path().join("main.exe")
    } else {
        temp_dir.path().join("main")
    };
    
    // Compile C code
    let compile_output = timeout(EXECUTION_TIMEOUT, async {
        Command::new("gcc")
            .arg(&c_file)
            .arg("-o")
            .arg(&exe_file)
            .arg("-std=c99")
            .arg("-Wall")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Compilation timeout".to_string())?
    .map_err(|e| format!("Compilation failed: {}", e))?;
    
    if !compile_output.status.success() {
        return Err(format!("Compilation error: {}", 
            String::from_utf8_lossy(&compile_output.stderr)));
    }
    
    // Execute compiled binary
    let run_output = timeout(EXECUTION_TIMEOUT, async {
        Command::new(&exe_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Execution timeout".to_string())?
    .map_err(|e| format!("Execution failed: {}", e))?;
    
    if !run_output.status.success() {
        return Err(format!("Runtime error: {}", 
            String::from_utf8_lossy(&run_output.stderr)));
    }
    
    let output = String::from_utf8_lossy(&run_output.stdout);
    if output.len() > MAX_OUTPUT_SIZE {
        return Err("Output too large".to_string());
    }
    
    Ok(output.to_string())
}

// Execute C++ code
async fn execute_cpp_code(code: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    let cpp_file = temp_dir.path().join("main.cpp");
    
    fs::write(&cpp_file, code)
        .map_err(|e| format!("Failed to write code: {}", e))?;
    
    let exe_file = if cfg!(target_os = "windows") {
        temp_dir.path().join("main.exe")
    } else {
        temp_dir.path().join("main")
    };
    
    let compile_output = timeout(EXECUTION_TIMEOUT, async {
        Command::new("g++")
            .arg(&cpp_file)
            .arg("-o")
            .arg(&exe_file)
            .arg("-std=c++17")
            .arg("-Wall")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Compilation timeout".to_string())?
    .map_err(|e| format!("Compilation failed: {}", e))?;
    
    if !compile_output.status.success() {
        return Err(format!("Compilation error: {}", 
            String::from_utf8_lossy(&compile_output.stderr)));
    }
    
    let run_output = timeout(EXECUTION_TIMEOUT, async {
        Command::new(&exe_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Execution timeout".to_string())?
    .map_err(|e| format!("Execution failed: {}", e))?;
    
    if !run_output.status.success() {
        return Err(format!("Runtime error: {}", 
            String::from_utf8_lossy(&run_output.stderr)));
    }
    
    let output = String::from_utf8_lossy(&run_output.stdout);
    if output.len() > MAX_OUTPUT_SIZE {
        return Err("Output too large".to_string());
    }
    
    Ok(output.to_string())
}

// Execute JavaScript code
async fn execute_javascript_code(code: &str) -> Result<String, String> {
    let output = timeout(EXECUTION_TIMEOUT, async {
        Command::new("node")
            .arg("-e")
            .arg(code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Execution timeout".to_string())?
    .map_err(|e| format!("Failed to execute Node.js: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Runtime error: {}", 
            String::from_utf8_lossy(&output.stderr)));
    }
    
    let result = String::from_utf8_lossy(&output.stdout);
    if result.len() > MAX_OUTPUT_SIZE {
        return Err("Output too large".to_string());
    }
    
    Ok(result.to_string())
}

// Execute Go code
async fn execute_go_code(code: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    let go_file = temp_dir.path().join("main.go");
    
    // Ensure code has package main and func main
    let wrapped_code = if !code.contains("package main") {
        format!("package main\n\nimport \"fmt\"\n\nfunc main() {{\n{}\n}}", code)
    } else {
        code.to_string()
    };
    
    fs::write(&go_file, wrapped_code)
        .map_err(|e| format!("Failed to write code: {}", e))?;
    
    // We can run directly with 'go run', but compiling matches other flows better
    // 'go run' is simpler for one-off execution though
    let run_output = timeout(EXECUTION_TIMEOUT, async {
        Command::new("go")
            .arg("run")
            .arg(&go_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    })
    .await
    .map_err(|_| "Execution timeout".to_string())?
    .map_err(|e| format!("Execution failed: {}", e))?;
    
    if !run_output.status.success() {
        return Err(format!("Runtime error: {}", 
            String::from_utf8_lossy(&run_output.stderr)));
    }
    
    let output = String::from_utf8_lossy(&run_output.stdout);
    if output.len() > MAX_OUTPUT_SIZE {
        return Err("Output too large".to_string());
    }
    
    Ok(output.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to create database pool");

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    println!("Multi-language compiler server starting on http://0.0.0.0:{}", port);
    println!("Supported languages: custom, rust, python, c, cpp, javascript, go");
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://127.0.0.1:3000")
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(3600);
            
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::JsonConfig::default().limit(1024 * 1024)) 
            .route("/compile", web::post().to(compile_handler))
            .route("/health", web::get().to(|| async { HttpResponse::Ok().json("OK") }))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}