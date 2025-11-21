//! Response macros

/// Macro for creating JSON responses
#[macro_export]
macro_rules! json_response {
    ($data:expr) => {
        $crate::Response::json(&$data)
    };
    ($data:expr, $status:expr) => {
        $crate::Response::json(&$data).status($status)
    };
}

/// Macro for creating redirect responses
#[macro_export]
macro_rules! redirect {
    ($url:expr) => {
        $crate::Response::redirect($url)
    };
    ($url:expr, with: {$($key:expr => $value:expr),*}) => {
        {
            let mut builder = $crate::Response::redirect($url);
            $(
                builder = builder.with($key, $value);
            )*
            builder
        }
    };
}

/// Macro for creating download responses
#[macro_export]
macro_rules! download {
    ($path:expr) => {
        $crate::Response::download($path, std::path::Path::new($path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("download"))
    };
    ($path:expr, $filename:expr) => {
        $crate::Response::download($path, $filename)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Response;
    use serde_json::json;

    #[test]
    fn test_json_response_macro() {
        let _response = json_response!(json!({"test": true}));
    }

    #[test]
    fn test_redirect_macro() {
        let _response = redirect!("/home");
    }

    #[test]
    fn test_download_macro() {
        let _response = download!("/path/to/file.pdf", "document.pdf");
    }
}
