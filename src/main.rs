// src/main.rs - Performance-optimized version with all fixes applied
use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use serde::{Deserialize, Serialize};
use actix_cors::Cors;
use std::collections::HashMap;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use dotenv::dotenv;
use std::env;
use std::process::Stdio;
use tokio::process::Command;  // FIX: Use async tokio::process instead of std::process
use tokio::time::{timeout, Duration};
use std::fs;
use std::sync::{OnceLock, RwLock};

mod lexer;
mod parser;
mod evaluator;
mod object;

// ─── Python command detection (cached at startup) ───
static PYTHON_CMD: OnceLock<Option<String>> = OnceLock::new();

fn detect_python_command() -> &'static Option<String> {
    PYTHON_CMD.get_or_init(|| {
        for cmd in ["python3", "python", "py"] {
            if std::process::Command::new(cmd)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
            {
                log::info!("Detected Python command: {}", cmd);
                return Some(cmd.to_string());
            }
        }
        log::warn!("No Python command found");
        None
    })
}

// ─── Simple response cache ───
lazy_static::lazy_static! {
    static ref RESPONSE_CACHE: RwLock<HashMap<u64, CachedResponse>> =
        RwLock::new(HashMap::new());
}

#[derive(Clone)]
struct CachedResponse {
    result: Option<String>,
    error: Option<String>,
    execution_time_ms: u64,
}

fn hash_code(code: &str, language: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    code.hash(&mut hasher);
    language.hash(&mut hasher);
    hasher.finish()
}

// ─── Temp dir helper: prefer tmpfs/ramdisk, fall back to OS temp ───
fn create_fast_temp_dir() -> Result<tempfile::TempDir, String> {
    // Try /dev/shm (Linux shared memory) first for speed
    if cfg!(target_os = "linux") {
        if let Ok(dir) = tempfile::Builder::new()
            .prefix("cc_")
            .tempdir_in("/dev/shm")
        {
            return Ok(dir);
        }
    }
    tempfile::tempdir().map_err(|e| format!("Failed to create temp directory: {}", e))
}

// ─── Types ───

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
    language: String,
    #[serde(default)]
    input: String,  // stdin input for the program
}

#[derive(Serialize, Clone)]
struct CompileResponse {
    result: Option<String>,
    error: Option<String>,
    execution_time_ms: Option<u64>,
}

// ─── Constants ───
const EXECUTION_TIMEOUT: Duration = Duration::from_secs(10);
const COMPILATION_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_OUTPUT_SIZE: usize = 50_000;   // 50KB max output
const MAX_CODE_SIZE: usize = 100_000;    // 100KB max code
const MAX_CACHE_SIZE: usize = 500;       // Max cached responses

// ─── Main handler ───

async fn compile_handler(req: web::Json<CompileRequest>, pool: web::Data<PgPool>) -> impl Responder {
    let start_time = std::time::Instant::now();
    let code = &req.code;
    let language = &req.language.to_lowercase();
    let input = &req.input;

    // Security: input validation
    if code.len() > MAX_CODE_SIZE {
        return HttpResponse::BadRequest().json(CompileResponse {
            result: None,
            error: Some(format!("Code too large (max {}KB)", MAX_CODE_SIZE / 1000)),
            execution_time_ms: Some(start_time.elapsed().as_millis() as u64),
        });
    }

    // Check cache first (skip for custom language since it has global state issues)
    let cache_key = hash_code(code, language);
    if language != "custom" && input.is_empty() {
        if let Ok(cache) = RESPONSE_CACHE.read() {
            if let Some(cached) = cache.get(&cache_key) {
                return HttpResponse::Ok().json(CompileResponse {
                    result: cached.result.clone(),
                    error: cached.error.clone(),
                    execution_time_ms: Some(cached.execution_time_ms),
                });
            }
        }
    }

    let result = match language.as_str() {
        "custom" => execute_custom_language(code).await,
        "rust" => execute_rust_code(code, input).await,
        "python" => execute_python_code(code, input).await,
        "c" => execute_c_code(code, input).await,
        "cpp" | "c++" => execute_cpp_code(code, input).await,
        "javascript" | "js" => execute_javascript_code(code, input).await,
        "go" => execute_go_code(code, input).await,
        "java" => execute_java_code(code, input).await,
        _ => Err("Unsupported language. Use: custom, rust, python, c, cpp, javascript, go, java".to_string()),
    };

    let execution_time = start_time.elapsed().as_millis() as u64;

    let response = match result {
        Ok(output) => {
            // FIX: Async DB write - fire and forget, don't block response
            let pool_clone = pool.get_ref().clone();
            let code_clone = code.to_string();
            let output_clone = output.clone();
            let lang_clone = language.to_string();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "INSERT INTO executions (code, result, status, execution_time_ms, language) VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(&code_clone)
                .bind(Some(&output_clone))
                .bind(ExecutionStatus::Success as ExecutionStatus)
                .bind(execution_time as i32)
                .bind(&lang_clone)
                .execute(&pool_clone)
                .await;
            });

            CompileResponse {
                result: Some(output),
                error: None,
                execution_time_ms: Some(execution_time),
            }
        }
        Err(error) => {
            // FIX: Async DB write
            let pool_clone = pool.get_ref().clone();
            let code_clone = code.to_string();
            let error_clone = error.clone();
            let lang_clone = language.to_string();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "INSERT INTO executions (code, error, status, execution_time_ms, language) VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(&code_clone)
                .bind(Some(&error_clone))
                .bind(ExecutionStatus::Error as ExecutionStatus)
                .bind(execution_time as i32)
                .bind(&lang_clone)
                .execute(&pool_clone)
                .await;
            });

            CompileResponse {
                result: None,
                error: Some(error),
                execution_time_ms: Some(execution_time),
            }
        }
    };

    // Cache the response
    if language != "custom" && input.is_empty() {
        if let Ok(mut cache) = RESPONSE_CACHE.write() {
            if cache.len() >= MAX_CACHE_SIZE {
                cache.clear(); // Simple eviction
            }
            cache.insert(cache_key, CachedResponse {
                result: response.result.clone(),
                error: response.error.clone(),
                execution_time_ms: execution_time,
            });
        }
    }

    HttpResponse::Ok().json(response)
}

// ─── Helper: run a compiled binary with optional stdin ───

async fn run_binary(exe_path: &std::path::Path, input: &str) -> Result<String, String> {
    let mut child = Command::new(exe_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Execution failed: {}", e))?;

    // Write stdin if provided
    if !input.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(input.as_bytes()).await;
            drop(stdin); // Close stdin
        }
    }

    let output = timeout(EXECUTION_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "Execution timeout (10s limit)".to_string())?
        .map_err(|e| format!("Execution failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("Runtime error:\n{}", String::from_utf8_lossy(&output.stderr)));
    }

    let result = String::from_utf8_lossy(&output.stdout);
    if result.len() > MAX_OUTPUT_SIZE {
        return Err(format!("Output too large (max {}KB)", MAX_OUTPUT_SIZE / 1000));
    }

    Ok(result.to_string())
}

// ─── Helper: compile code with a compiler command ───

async fn compile_code(
    compiler: &str,
    source_file: &std::path::Path,
    exe_file: &std::path::Path,
    extra_args: &[&str],
) -> Result<(), String> {
    let mut cmd = Command::new(compiler);
    cmd.arg(source_file).arg("-o").arg(exe_file);
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = timeout(COMPILATION_TIMEOUT, cmd.output())
        .await
        .map_err(|_| "Compilation timeout (15s limit)".to_string())?
        .map_err(|e| format!("Compilation failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("Compilation error:\n{}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(())
}

// ─── Execute custom language (your interpreter) ───

async fn execute_custom_language(code: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(code)?;
    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse_program()?;
    let mut env = HashMap::new();
    
    let result = evaluator::evaluate(&ast, &mut env)?;
    let output = evaluator::get_output();
    
    if !output.is_empty() {
        Ok(output.trim_end().to_string())
    } else {
        match result {
            object::Object::Null => Ok(String::new()),
            object::Object::String(s) if s.is_empty() => Ok(String::new()),
            other => Ok(other.to_string())
        }
    }
}

// ─── Rust ───

async fn execute_rust_code(code: &str, input: &str) -> Result<String, String> {
    let temp_dir = create_fast_temp_dir()?;
    let rust_file = temp_dir.path().join("main.rs");
    
    let wrapped_code = if !code.contains("fn main") {
        format!("fn main() {{\n{}\n}}", code)
    } else {
        code.to_string()
    };
    
    fs::write(&rust_file, wrapped_code).map_err(|e| format!("Failed to write code: {}", e))?;
    
    let exe_file = temp_dir.path().join(if cfg!(target_os = "windows") { "main.exe" } else { "main" });
    
    compile_code("rustc", &rust_file, &exe_file, &["--edition=2021", "-A", "warnings"]).await?;
    run_binary(&exe_file, input).await
}

// ─── Python ───

async fn execute_python_code(code: &str, input: &str) -> Result<String, String> {
    let python_cmd = detect_python_command()
        .as_deref()
        .ok_or("Python is not installed. Please install Python or use a different language.")?;

    let mut child = Command::new(python_cmd)
        .arg("-c")
        .arg(code)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute Python: {}", e))?;

    if !input.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(input.as_bytes()).await;
            drop(stdin);
        }
    }

    let output = timeout(EXECUTION_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "Execution timeout (10s limit)".to_string())?
        .map_err(|e| format!("Failed to execute Python: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Python error:\n{}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let result = String::from_utf8_lossy(&output.stdout);
    if result.len() > MAX_OUTPUT_SIZE {
        return Err(format!("Output too large (max {}KB)", MAX_OUTPUT_SIZE / 1000));
    }
    
    Ok(result.to_string())
}

// ─── C ───

async fn execute_c_code(code: &str, input: &str) -> Result<String, String> {
    let temp_dir = create_fast_temp_dir()?;
    let c_file = temp_dir.path().join("main.c");
    fs::write(&c_file, code).map_err(|e| format!("Failed to write code: {}", e))?;
    let exe_file = temp_dir.path().join(if cfg!(target_os = "windows") { "main.exe" } else { "main" });
    
    compile_code("gcc", &c_file, &exe_file, &["-std=c11", "-O2", "-lm", "-Wall"]).await?;
    run_binary(&exe_file, input).await
}

// ─── C++ ───

async fn execute_cpp_code(code: &str, input: &str) -> Result<String, String> {
    let temp_dir = create_fast_temp_dir()?;
    let cpp_file = temp_dir.path().join("main.cpp");
    fs::write(&cpp_file, code).map_err(|e| format!("Failed to write code: {}", e))?;
    let exe_file = temp_dir.path().join(if cfg!(target_os = "windows") { "main.exe" } else { "main" });
    
    compile_code("g++", &cpp_file, &exe_file, &["-std=c++17", "-O2", "-Wall"]).await?;
    run_binary(&exe_file, input).await
}

// ─── JavaScript ───

async fn execute_javascript_code(code: &str, input: &str) -> Result<String, String> {
    let mut child = Command::new("node")
        .arg("-e")
        .arg(code)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute Node.js: {}", e))?;

    if !input.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(input.as_bytes()).await;
            drop(stdin);
        }
    }

    let output = timeout(EXECUTION_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "Execution timeout (10s limit)".to_string())?
        .map_err(|e| format!("Failed to execute Node.js: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Runtime error:\n{}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let result = String::from_utf8_lossy(&output.stdout);
    if result.len() > MAX_OUTPUT_SIZE {
        return Err(format!("Output too large (max {}KB)", MAX_OUTPUT_SIZE / 1000));
    }
    
    Ok(result.to_string())
}

// ─── Go ───

async fn execute_go_code(code: &str, input: &str) -> Result<String, String> {
    let temp_dir = create_fast_temp_dir()?;
    let go_file = temp_dir.path().join("main.go");
    
    let wrapped_code = if !code.contains("package main") {
        format!("package main\n\nimport \"fmt\"\n\nfunc main() {{\n{}\n}}", code)
    } else {
        code.to_string()
    };
    
    fs::write(&go_file, wrapped_code).map_err(|e| format!("Failed to write code: {}", e))?;
    
    // Use 'go run' for simplicity, spawn with stdin support
    let mut child = Command::new("go")
        .arg("run")
        .arg(&go_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Execution failed: {}", e))?;

    if !input.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(input.as_bytes()).await;
            drop(stdin);
        }
    }

    let output = timeout(EXECUTION_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "Execution timeout (10s limit)".to_string())?
        .map_err(|e| format!("Execution failed: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Runtime error:\n{}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let result = String::from_utf8_lossy(&output.stdout);
    if result.len() > MAX_OUTPUT_SIZE {
        return Err(format!("Output too large (max {}KB)", MAX_OUTPUT_SIZE / 1000));
    }
    
    Ok(result.to_string())
}

// ─── Java (NEW) ───

async fn execute_java_code(code: &str, input: &str) -> Result<String, String> {
    let temp_dir = create_fast_temp_dir()?;
    
    // Extract class name from code, default to Main
    let class_name = extract_java_class_name(code).unwrap_or("Main".to_string());
    let java_file = temp_dir.path().join(format!("{}.java", class_name));
    
    fs::write(&java_file, code).map_err(|e| format!("Failed to write code: {}", e))?;
    
    // Compile
    let compile_output = timeout(COMPILATION_TIMEOUT,
        Command::new("javac")
            .arg(&java_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    )
    .await
    .map_err(|_| "Compilation timeout (15s limit)".to_string())?
    .map_err(|e| format!("Compilation failed: {}", e))?;
    
    if !compile_output.status.success() {
        return Err(format!("Compilation error:\n{}", String::from_utf8_lossy(&compile_output.stderr)));
    }
    
    // Run
    let mut child = Command::new("java")
        .arg("-cp")
        .arg(temp_dir.path())
        .arg(&class_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Execution failed: {}", e))?;

    if !input.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(input.as_bytes()).await;
            drop(stdin);
        }
    }

    let output = timeout(EXECUTION_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "Execution timeout (10s limit)".to_string())?
        .map_err(|e| format!("Execution failed: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Runtime error:\n{}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let result = String::from_utf8_lossy(&output.stdout);
    if result.len() > MAX_OUTPUT_SIZE {
        return Err(format!("Output too large (max {}KB)", MAX_OUTPUT_SIZE / 1000));
    }
    
    Ok(result.to_string())
}

fn extract_java_class_name(code: &str) -> Option<String> {
    for line in code.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("public class ") {
            if let Some(name) = rest.split_whitespace().next() {
                let name = name.trim_end_matches('{');
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

// ─── Health & languages endpoint ───

async fn health_handler() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": "0.3.0",
        "languages": ["custom", "rust", "python", "c", "cpp", "javascript", "go", "java"]
    }))
}

// ─── Main ───

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Detect Python at startup
    detect_python_command();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // FIX: Proper connection pool configuration
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .connect(&database_url)
        .await
        .expect("Failed to create database pool");

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    log::info!("CodeConnect compiler server starting on http://0.0.0.0:{}", port);
    log::info!("Supported languages: custom, rust, python, c, cpp, javascript, go, java");
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://127.0.0.1:3000")
            // Allow any Vercel or Render deployed frontend
            .allowed_origin_fn(|origin, _req_head| {
                let origin = origin.as_bytes();
                let origin_str = std::str::from_utf8(origin).unwrap_or("");
                origin_str.ends_with(".vercel.app")
                    || origin_str.ends_with(".onrender.com")
            })
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(3600);
            
        App::new()
            .wrap(Logger::new("%r %s %D ms"))
            .wrap(cors)
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::JsonConfig::default().limit(1024 * 1024)) 
            .route("/compile", web::post().to(compile_handler))
            .route("/health", web::get().to(health_handler))
    })
    .workers(4)
    .bind(("0.0.0.0", port))?
    .run()
    .await
}