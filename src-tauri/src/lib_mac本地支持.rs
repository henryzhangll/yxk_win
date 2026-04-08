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
            
            // 统一使用 reqwest
            let client = reqwest::blocking::Client::builder()
                .build()
                .unwrap();
            
            let mut request_builder = client.request(
                reqwest::Method::from_bytes(method.as_bytes()).unwrap(),
                &target_url
            );
            
            // 复制请求头
            for (key, value) in request.headers() {
                if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&value.to_str().unwrap_or("")) {
                    if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_str().as_bytes()) {
                        request_builder = request_builder.header(header_name, header_value);
                    }
                }
            }
            
            // 添加请求体
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
                    
                    tauri::http::Response::builder()
                        .status(status)
                        .header("Content-Type", "application/json")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(body.into_bytes())
                        .unwrap()
                }
                Err(e) => {
                    println!("❌ 代理失败: {}", e);
                    tauri::http::Response::builder()
                        .status(500)
                        .body(e.to_string().into_bytes())
                        .unwrap()
                }
            }
        })
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            
            // 注入劫持脚本（放在 index.html 中更稳定，但这里也保留）
            let script = r#"
                (function() {
                    if (window.__proxy_installed__) {
                        return;
                    }
                    window.__proxy_installed__ = true;
                    
                    console.log('🚀 启动代理劫持');
                    
                    const OriginalXHR = window.XMLHttpRequest;
                    
                    window.XMLHttpRequest = function() {
                        const xhr = new OriginalXHR();
                        const originalOpen = xhr.open;
                        
                        xhr.open = function(method, url, async, user, password) {
                            if (url && (url.includes('/api') || url.includes('/wxcx'))) {
                                const proxyUrl = 'proxy://localhost' + url;
                                console.log('🔄 劫持:', url, '->', proxyUrl);
                                return originalOpen.call(this, method, proxyUrl, async, user, password);
                            }
                            return originalOpen.call(this, method, url, async, user, password);
                        };
                        
                        return xhr;
                    };
                    
                    window.XMLHttpRequest.prototype = OriginalXHR.prototype;
                    
                    const originalFetch = window.fetch;
                    window.fetch = function(url, options) {
                        if (typeof url === 'string' && (url.includes('/api') || url.includes('/wxcx'))) {
                            const proxyUrl = 'proxy://localhost' + url;
                            console.log('🔄 Fetch 劫持:', url, '->', proxyUrl);
                            return originalFetch(proxyUrl, options);
                        }
                        return originalFetch(url, options);
                    };
                    
                    console.log('✅ 代理劫持已启用');
                })();
            "#;
            
            let _ = window.eval(script);
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}