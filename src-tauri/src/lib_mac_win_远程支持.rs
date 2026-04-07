use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .register_uri_scheme_protocol("proxy", |_app, request| {
            let uri = request.uri().to_string();
            let path = uri.replacen("proxy://localhost", "", 1);
            
            let target_url = if path.starts_with("/api") || path.starts_with("/wxcx") {
                format!("https://test.zk69.cn{}", path)
            } else {
                return tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap();
            };
            
            let method = request.method().to_string();
            println!("🔄 代理请求: {} {} -> {}", method, path, target_url);
            
            let body_bytes = request.body().clone();
            
            if !body_bytes.is_empty() {
                if let Ok(body_str) = String::from_utf8(body_bytes.clone()) {
                    println!("📦 请求体: {}", body_str);
                }
            }
            
            // Mac: 使用 reqwest
            #[cfg(target_os = "macos")]
            {
                let client = reqwest::blocking::Client::builder()
                    .build()
                    .unwrap();
                
                let mut request_builder = client.request(
                    reqwest::Method::from_bytes(method.as_bytes()).unwrap(),
                    &target_url
                );
                
                for (key, value) in request.headers() {
                    if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&value.to_str().unwrap_or("")) {
                        if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_str().as_bytes()) {
                            request_builder = request_builder.header(header_name, header_value);
                        }
                    }
                }
                
                let request_builder = if !body_bytes.is_empty() {
                    request_builder.body(body_bytes)
                } else {
                    request_builder
                };
                
                match request_builder.send() {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let body = response.text().unwrap_or_default();
                        println!("✅ 代理成功: 状态码 {}", status);
                        
                        return tauri::http::Response::builder()
                            .status(status)
                            .header("Content-Type", "application/json")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(body.into_bytes())
                            .unwrap();
                    }
                    Err(e) => {
                        println!("❌ 代理失败: {}", e);
                        return tauri::http::Response::builder()
                            .status(500)
                            .body(e.to_string().into_bytes())
                            .unwrap();
                    }
                }
            }
            
            // Windows: 使用 curl
            #[cfg(target_os = "windows")]
            {
                use std::process::Command;
                
                let mut cmd = Command::new("curl");
                cmd.arg("-X").arg(&method);
                cmd.arg("-s");
                cmd.arg("-H").arg("Content-Type: application/json");
                
                if !body_bytes.is_empty() {
                    if let Ok(body_str) = String::from_utf8(body_bytes) {
                        cmd.arg("-d").arg(&body_str);
                    }
                }
                
                cmd.arg(&target_url);
                
                match cmd.output() {
                    Ok(output) => {
                        if output.status.success() {
                            let body = String::from_utf8_lossy(&output.stdout).to_string();
                            println!("✅ 代理成功");
                            return tauri::http::Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .header("Access-Control-Allow-Origin", "*")
                                .body(body.into_bytes())
                                .unwrap();
                        } else {
                            let err = format!("curl failed: {:?}", output.status);
                            println!("❌ {}", err);
                            return tauri::http::Response::builder()
                                .status(500)
                                .body(err.into_bytes())
                                .unwrap();
                        }
                    }
                    Err(e) => {
                        println!("❌ 代理失败: {}", e);
                        return tauri::http::Response::builder()
                            .status(500)
                            .body(e.to_string().into_bytes())
                            .unwrap();
                    }
                }
            }
            
            // Linux: 使用 curl
            #[cfg(target_os = "linux")]
            {
                use std::process::Command;
                
                let mut cmd = Command::new("curl");
                cmd.arg("-X").arg(&method);
                cmd.arg("-s");
                cmd.arg("-H").arg("Content-Type: application/json");
                
                if !body_bytes.is_empty() {
                    if let Ok(body_str) = String::from_utf8(body_bytes) {
                        cmd.arg("-d").arg(&body_str);
                    }
                }
                
                cmd.arg(&target_url);
                
                match cmd.output() {
                    Ok(output) => {
                        if output.status.success() {
                            let body = String::from_utf8_lossy(&output.stdout).to_string();
                            println!("✅ 代理成功");
                            return tauri::http::Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .header("Access-Control-Allow-Origin", "*")
                                .body(body.into_bytes())
                                .unwrap();
                        } else {
                            let err = format!("curl failed: {:?}", output.status);
                            println!("❌ {}", err);
                            return tauri::http::Response::builder()
                                .status(500)
                                .body(err.into_bytes())
                                .unwrap();
                        }
                    }
                    Err(e) => {
                        println!("❌ 代理失败: {}", e);
                        return tauri::http::Response::builder()
                            .status(500)
                            .body(e.to_string().into_bytes())
                            .unwrap();
                    }
                }
            }
            
            // 默认返回（不会执行到这里）
            return tauri::http::Response::builder()
                .status(500)
                .body(b"Unsupported platform".to_vec())
                .unwrap();
        })
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            
            #[cfg(debug_assertions)]
            {
                let _ = window.eval("window.__TAURI_INTERNALS__.invoke('plugin:devtools|open')");
            }
            
            let script = r#"
                (function() {
                    console.log('🚀 启动代理');
                    
                    const OriginalXHR = window.XMLHttpRequest;
                    
                    function ProxyXHR() {
                        const xhr = new OriginalXHR();
                        let _method, _url;
                        let _headers = {};
                        
                        const originalOpen = xhr.open;
                        const originalSend = xhr.send;
                        const originalSetRequestHeader = xhr.setRequestHeader;
                        
                        xhr.setRequestHeader = function(header, value) {
                            _headers[header] = value;
                            return originalSetRequestHeader.call(this, header, value);
                        };
                        
                        xhr.open = function(method, url, async = true, user, password) {
                            _method = method;
                            _url = url;
                            
                            if (url && (url.includes('/api') || url.includes('/wxcx'))) {
                                xhr._useProxy = true;
                                const proxyUrl = 'proxy://localhost' + url;
                                console.log('🔄 代理:', method, url);
                                return originalOpen.call(this, method, proxyUrl, async, user, password);
                            }
                            
                            return originalOpen.call(this, method, url, async, user, password);
                        };
                        
                        xhr.send = function(body) {
                            if (xhr._useProxy) {
                                for (const [key, value] of Object.entries(_headers)) {
                                    originalSetRequestHeader.call(this, key, value);
                                }
                                console.log('📦 请求体:', body);
                            }
                            return originalSend.call(this, body);
                        };
                        
                        return xhr;
                    }
                    
                    window.XMLHttpRequest = ProxyXHR;
                    window.XMLHttpRequest.prototype = OriginalXHR.prototype;
                    
                    const originalFetch = window.fetch;
                    window.fetch = function(url, options = {}) {
                        if (typeof url === 'string' && (url.includes('/api') || url.includes('/wxcx'))) {
                            const proxyUrl = 'proxy://localhost' + url;
                            console.log('🔄 fetch 代理:', url);
                            return originalFetch(proxyUrl, options);
                        }
                        return originalFetch(url, options);
                    };
                    
                    console.log('✅ 代理已启用');
                })();
            "#;
            
            let _ = window.eval(script);
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}