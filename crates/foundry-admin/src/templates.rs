//! Template engine for admin panel

use crate::config::AdminConfig;
use serde::Serialize;
use tera::{Context, Tera};

/// Template engine wrapper
pub struct TemplateEngine {
    tera: Tera,
    config: AdminConfig,
}

impl TemplateEngine {
    pub fn new(config: AdminConfig) -> Self {
        // In production, load templates from files
        let mut tera = Tera::default();

        // Register default templates
        Self::register_default_templates(&mut tera);

        Self { tera, config }
    }

    fn register_default_templates(tera: &mut Tera) {
        // Dashboard template
        tera.add_raw_template(
            "dashboard.html",
            include_str!("templates/dashboard.html"),
        )
        .ok();

        // Login template
        tera.add_raw_template("login.html", include_str!("templates/login.html"))
            .ok();

        // Resource list template
        tera.add_raw_template(
            "resource_list.html",
            include_str!("templates/resource_list.html"),
        )
        .ok();

        // Resource form template
        tera.add_raw_template(
            "resource_form.html",
            include_str!("templates/resource_form.html"),
        )
        .ok();

        // Layout template
        tera.add_raw_template("layout.html", include_str!("templates/layout.html"))
            .ok();
    }

    pub fn render<T: Serialize>(
        &self,
        template: &str,
        context: &T,
    ) -> anyhow::Result<String> {
        let mut ctx = Context::from_serialize(context)?;
        ctx.insert("config", &self.config);
        Ok(self.tera.render(template, &ctx)?)
    }

    pub fn render_dashboard(&self, data: impl Serialize) -> anyhow::Result<String> {
        self.render("dashboard.html", &data)
    }

    pub fn render_login(&self, error: Option<String>) -> anyhow::Result<String> {
        #[derive(Serialize)]
        struct LoginContext {
            error: Option<String>,
        }
        self.render("login.html", &LoginContext { error })
    }
}
