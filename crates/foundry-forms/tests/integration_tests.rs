//! Integration tests for foundry-forms

use foundry_forms::{Field, Form, FormData, Theme};

#[test]
fn test_form_builder() {
    let form = Form::new("test_form")
        .action("/submit")
        .field(Field::text("name").label("Name").required().build())
        .field(Field::email("email").label("Email").required().build())
        .submit("Submit")
        .build();

    assert_eq!(form.name, "test_form");
    assert_eq!(form.action, "/submit");
    assert_eq!(form.fields.len(), 2);
}

#[test]
fn test_form_validation_required() {
    let form = Form::new("test_form")
        .field(Field::text("name").label("Name").required().build())
        .build();

    let mut data = FormData::new();
    data.insert("name", "");

    let result = form.validate(&data);
    assert!(result.is_err());
}

#[test]
fn test_form_validation_email() {
    let form = Form::new("test_form")
        .field(
            Field::email("email")
                .label("Email")
                .required()
                .build()
        )
        .build();

    let mut data = FormData::new();
    data.insert("email", "invalid-email");

    let result = form.validate(&data);
    assert!(result.is_err());

    let mut valid_data = FormData::new();
    valid_data.insert("email", "test@example.com");

    let result = form.validate(&valid_data);
    assert!(result.is_ok());
}

#[test]
fn test_form_validation_min_length() {
    let form = Form::new("test_form")
        .field(
            Field::text("password")
                .label("Password")
                .min_length(8)
                .build()
        )
        .build();

    let mut data = FormData::new();
    data.insert("password", "short");

    let result = form.validate(&data);
    assert!(result.is_err());

    let mut valid_data = FormData::new();
    valid_data.insert("password", "longenough");

    let result = form.validate(&valid_data);
    assert!(result.is_ok());
}

#[test]
fn test_form_render_tailwind() {
    let form = Form::new("test_form")
        .action("/submit")
        .field(Field::text("name").label("Name").build())
        .submit("Submit")
        .build();

    let html = form.render(Theme::Tailwind);
    assert!(html.is_ok());

    let html_content = html.unwrap();
    assert!(html_content.contains("form"));
    assert!(html_content.contains("name"));
    assert!(html_content.contains("Submit"));
}

#[test]
fn test_form_render_bootstrap() {
    let form = Form::new("test_form")
        .action("/submit")
        .field(Field::text("name").label("Name").build())
        .submit("Submit")
        .build();

    let html = form.render(Theme::Bootstrap);
    assert!(html.is_ok());

    let html_content = html.unwrap();
    assert!(html_content.contains("form-control"));
}
