//! HTML Templates for Authentication
//!
//! Simple HTML templates for authentication pages

/// Base template trait
pub trait Template {
    fn render(&self) -> String;
}

/// Login Template
pub struct LoginTemplate {}

impl Template for LoginTemplate {
    fn render(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login - Foundry</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: system-ui, -apple-system, sans-serif; background: #f3f4f6; }
        .container { max-width: 400px; margin: 100px auto; padding: 20px; }
        .card { background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        h1 { margin-bottom: 30px; font-size: 24px; text-align: center; }
        .form-group { margin-bottom: 20px; }
        label { display: block; margin-bottom: 8px; font-weight: 500; color: #374151; }
        input { width: 100%; padding: 10px 12px; border: 1px solid #d1d5db; border-radius: 4px; font-size: 14px; }
        input:focus { outline: none; border-color: #3b82f6; }
        .checkbox-group { display: flex; align-items: center; margin-bottom: 20px; }
        .checkbox-group input { width: auto; margin-right: 8px; }
        button { width: 100%; padding: 12px; background: #3b82f6; color: white; border: none; border-radius: 4px; font-size: 16px; font-weight: 500; cursor: pointer; }
        button:hover { background: #2563eb; }
        .links { margin-top: 20px; text-align: center; font-size: 14px; }
        .links a { color: #3b82f6; text-decoration: none; }
        .links a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <h1>Login</h1>
            <form method="POST" action="/login">
                <div class="form-group">
                    <label for="email">Email</label>
                    <input type="email" id="email" name="email" required autofocus>
                </div>
                <div class="form-group">
                    <label for="password">Password</label>
                    <input type="password" id="password" name="password" required>
                </div>
                <div class="checkbox-group">
                    <input type="checkbox" id="remember" name="remember" value="true">
                    <label for="remember" style="margin-bottom: 0; font-weight: normal;">Remember me</label>
                </div>
                <button type="submit">Login</button>
                <div class="links">
                    <a href="/password/forgot">Forgot password?</a> ·
                    <a href="/register">Create account</a>
                </div>
            </form>
        </div>
    </div>
</body>
</html>"#.to_string()
    }
}

/// Register Template
pub struct RegisterTemplate {}

impl Template for RegisterTemplate {
    fn render(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Register - Foundry</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: system-ui, -apple-system, sans-serif; background: #f3f4f6; }
        .container { max-width: 400px; margin: 100px auto; padding: 20px; }
        .card { background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        h1 { margin-bottom: 30px; font-size: 24px; text-align: center; }
        .form-group { margin-bottom: 20px; }
        label { display: block; margin-bottom: 8px; font-weight: 500; color: #374151; }
        input { width: 100%; padding: 10px 12px; border: 1px solid #d1d5db; border-radius: 4px; font-size: 14px; }
        input:focus { outline: none; border-color: #3b82f6; }
        button { width: 100%; padding: 12px; background: #3b82f6; color: white; border: none; border-radius: 4px; font-size: 16px; font-weight: 500; cursor: pointer; }
        button:hover { background: #2563eb; }
        .links { margin-top: 20px; text-align: center; font-size: 14px; }
        .links a { color: #3b82f6; text-decoration: none; }
        .links a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <h1>Create Account</h1>
            <form method="POST" action="/register">
                <div class="form-group">
                    <label for="name">Name</label>
                    <input type="text" id="name" name="name" required autofocus>
                </div>
                <div class="form-group">
                    <label for="email">Email</label>
                    <input type="email" id="email" name="email" required>
                </div>
                <div class="form-group">
                    <label for="password">Password</label>
                    <input type="password" id="password" name="password" required>
                </div>
                <div class="form-group">
                    <label for="password_confirmation">Confirm Password</label>
                    <input type="password" id="password_confirmation" name="password_confirmation" required>
                </div>
                <button type="submit">Register</button>
                <div class="links">
                    <a href="/login">Already have an account?</a>
                </div>
            </form>
        </div>
    </div>
</body>
</html>"#.to_string()
    }
}

/// Forgot Password Template
pub struct ForgotPasswordTemplate {}

impl Template for ForgotPasswordTemplate {
    fn render(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Forgot Password - Foundry</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: system-ui, -apple-system, sans-serif; background: #f3f4f6; }
        .container { max-width: 400px; margin: 100px auto; padding: 20px; }
        .card { background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        h1 { margin-bottom: 10px; font-size: 24px; text-align: center; }
        p { margin-bottom: 30px; text-align: center; color: #6b7280; font-size: 14px; }
        .form-group { margin-bottom: 20px; }
        label { display: block; margin-bottom: 8px; font-weight: 500; color: #374151; }
        input { width: 100%; padding: 10px 12px; border: 1px solid #d1d5db; border-radius: 4px; font-size: 14px; }
        input:focus { outline: none; border-color: #3b82f6; }
        button { width: 100%; padding: 12px; background: #3b82f6; color: white; border: none; border-radius: 4px; font-size: 16px; font-weight: 500; cursor: pointer; }
        button:hover { background: #2563eb; }
        .links { margin-top: 20px; text-align: center; font-size: 14px; }
        .links a { color: #3b82f6; text-decoration: none; }
        .links a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <h1>Forgot Password</h1>
            <p>Enter your email and we'll send you a password reset link.</p>
            <form method="POST" action="/password/forgot">
                <div class="form-group">
                    <label for="email">Email</label>
                    <input type="email" id="email" name="email" required autofocus>
                </div>
                <button type="submit">Send Reset Link</button>
                <div class="links">
                    <a href="/login">Back to login</a>
                </div>
            </form>
        </div>
    </div>
</body>
</html>"#.to_string()
    }
}

/// Reset Password Template
pub struct ResetPasswordTemplate {
    pub token: String,
    pub email: String,
}

impl Template for ResetPasswordTemplate {
    fn render(&self) -> String {
        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Reset Password - Foundry</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: system-ui, -apple-system, sans-serif; background: #f3f4f6; }}
        .container {{ max-width: 400px; margin: 100px auto; padding: 20px; }}
        .card {{ background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
        h1 {{ margin-bottom: 30px; font-size: 24px; text-align: center; }}
        .form-group {{ margin-bottom: 20px; }}
        label {{ display: block; margin-bottom: 8px; font-weight: 500; color: #374151; }}
        input {{ width: 100%; padding: 10px 12px; border: 1px solid #d1d5db; border-radius: 4px; font-size: 14px; }}
        input:focus {{ outline: none; border-color: #3b82f6; }}
        button {{ width: 100%; padding: 12px; background: #3b82f6; color: white; border: none; border-radius: 4px; font-size: 16px; font-weight: 500; cursor: pointer; }}
        button:hover {{ background: #2563eb; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <h1>Reset Password</h1>
            <form method="POST" action="/password/reset">
                <input type="hidden" name="token" value="{}">
                <div class="form-group">
                    <label for="email">Email</label>
                    <input type="email" id="email" name="email" value="{}" required>
                </div>
                <div class="form-group">
                    <label for="password">New Password</label>
                    <input type="password" id="password" name="password" required autofocus>
                </div>
                <div class="form-group">
                    <label for="password_confirmation">Confirm Password</label>
                    <input type="password" id="password_confirmation" name="password_confirmation" required>
                </div>
                <button type="submit">Reset Password</button>
            </form>
        </div>
    </div>
</body>
</html>"#, self.token, self.email)
    }
}
