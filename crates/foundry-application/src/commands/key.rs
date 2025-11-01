use async_trait::async_trait;
use foundry_plugins::{
    Command, CommandContext, CommandDescriptor, CommandHandle, CommandResult, ResponseFormat,
};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Command to generate application key
pub struct KeyGenerateCommand;

#[async_trait]
impl Command for KeyGenerateCommand {
    async fn execute(&self, ctx: CommandContext) -> anyhow::Result<CommandResult> {
        let force = ctx.args.contains(&"--force".to_string());
        let show = ctx.args.contains(&"--show".to_string());

        // Generate a new key (32 bytes random)
        let key = generate_key();
        let encoded_key = format!("base64:{}", base64_encode(&key));

        if show {
            return Ok(CommandResult::success(format!(
                "Generated key: {}",
                encoded_key
            )));
        }

        // Check if .env file exists
        let env_path = Path::new(".env");
        if !env_path.exists() {
            // Copy from .env.example if it exists
            if Path::new(".env.example").exists() {
                fs::copy(".env.example", ".env")?;
            } else {
                // Create new .env file
                let mut file = fs::File::create(".env")?;
                writeln!(file, "APP_KEY={}", encoded_key)?;
                return Ok(CommandResult::success(format!(
                    "Application key set successfully.\nKey: {}",
                    encoded_key
                )));
            }
        }

        // Read current .env content
        let env_content = fs::read_to_string(env_path)?;

        // Check if APP_KEY already exists and is not empty
        let has_key = env_content
            .lines()
            .any(|line| line.starts_with("APP_KEY=") && !line.trim_start_matches("APP_KEY=").is_empty());

        if has_key && !force {
            return Ok(CommandResult::error(
                "Application key already exists. Use --force to overwrite.".to_string(),
            ));
        }

        // Update or add APP_KEY in .env
        let new_content = if env_content.contains("APP_KEY=") {
            // Replace existing APP_KEY
            env_content
                .lines()
                .map(|line| {
                    if line.starts_with("APP_KEY=") {
                        format!("APP_KEY={}", encoded_key)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            // Add APP_KEY at the end
            format!("{}\nAPP_KEY={}", env_content.trim(), encoded_key)
        };

        // Write back to .env
        fs::write(env_path, new_content)?;

        Ok(CommandResult::success(format!(
            "Application key set successfully.\nKey: {}",
            encoded_key
        )))
    }
}

impl CommandHandle for KeyGenerateCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            name: "key:generate".to_string(),
            description: "Generate a new application key".to_string(),
            usage: "key:generate [--force] [--show]".to_string(),
            examples: vec![
                "key:generate".to_string(),
                "key:generate --force".to_string(),
                "key:generate --show".to_string(),
            ],
        }
    }
}

/// Command to show current application key
pub struct KeyShowCommand;

#[async_trait]
impl Command for KeyShowCommand {
    async fn execute(&self, _ctx: CommandContext) -> anyhow::Result<CommandResult> {
        // Try to get key from environment
        let key = std::env::var("APP_KEY").unwrap_or_default();

        if key.is_empty() {
            return Ok(CommandResult::error(
                "Application key is not set. Run 'key:generate' to generate a new key.".to_string(),
            ));
        }

        Ok(CommandResult::success(format!(
            "Current application key: {}",
            key
        )))
    }
}

impl CommandHandle for KeyShowCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            name: "key:show".to_string(),
            description: "Display the current application key".to_string(),
            usage: "key:show".to_string(),
            examples: vec!["key:show".to_string()],
        }
    }
}

/// Generate a random 32-byte key
fn generate_key() -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32).map(|_| rng.gen()).collect()
}

/// Base64 encode bytes
fn base64_encode(bytes: &[u8]) -> String {
    use std::io::Read;
    let mut buf = String::new();
    let mut encoder = base64::write::EncoderStringWriter::new(&mut buf, &base64::engine::general_purpose::STANDARD);
    encoder.write_all(bytes).unwrap();
    drop(encoder);
    buf
}

// Simple base64 encoding module
mod base64 {
    pub mod write {
        use std::io::{self, Write};

        pub struct EncoderStringWriter<'a> {
            output: &'a mut String,
            engine: &'a Engine,
        }

        impl<'a> EncoderStringWriter<'a> {
            pub fn new(output: &'a mut String, engine: &'a Engine) -> Self {
                Self { output, engine }
            }
        }

        impl<'a> Write for EncoderStringWriter<'a> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                let encoded = (self.engine.encode)(buf);
                self.output.push_str(&encoded);
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        pub trait Engine {
            fn encode(&self, input: &[u8]) -> String;
        }
    }

    pub mod engine {
        pub mod general_purpose {
            use super::super::write::Engine;

            pub struct Standard;

            pub const STANDARD: Standard = Standard;

            impl Engine for Standard {
                fn encode(&self, input: &[u8]) -> String {
                    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                    let mut result = String::new();
                    let mut i = 0;

                    while i + 2 < input.len() {
                        let b1 = input[i];
                        let b2 = input[i + 1];
                        let b3 = input[i + 2];

                        result.push(CHARSET[(b1 >> 2) as usize] as char);
                        result.push(CHARSET[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);
                        result.push(CHARSET[(((b2 & 0x0F) << 2) | (b3 >> 6)) as usize] as char);
                        result.push(CHARSET[(b3 & 0x3F) as usize] as char);

                        i += 3;
                    }

                    if i < input.len() {
                        let b1 = input[i];
                        result.push(CHARSET[(b1 >> 2) as usize] as char);

                        if i + 1 < input.len() {
                            let b2 = input[i + 1];
                            result.push(CHARSET[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);
                            result.push(CHARSET[((b2 & 0x0F) << 2) as usize] as char);
                            result.push('=');
                        } else {
                            result.push(CHARSET[((b1 & 0x03) << 4) as usize] as char);
                            result.push('=');
                            result.push('=');
                        }
                    }

                    result
                }
            }
        }
    }
}
