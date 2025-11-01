//! Helper macros for resource definitions

/// Macro to derive Resource trait for simple transformations
#[macro_export]
macro_rules! impl_resource {
    ($resource:ty, $model:ty, |$model_var:ident| $body:expr) => {
        impl $crate::Resource for $resource {
            type Model = $model;

            fn from_model($model_var: Self::Model) -> Self {
                $body
            }
        }
    };
}

/// Macro to create a resource with conditional fields
#[macro_export]
macro_rules! resource {
    (
        $name:ident {
            $(
                $field:ident : $value:expr
            ),* $(,)?
        }
    ) => {
        $name {
            $(
                $field: $value,
            )*
        }
    };
}

/// Macro for quick resource collection creation
#[macro_export]
macro_rules! collection {
    ($resource:ty, $models:expr) => {
        $crate::ResourceCollection::<$resource>::from_models($models)
    };

    ($resource:ty, $models:expr, paginate: ($page:expr, $per_page:expr, $total:expr)) => {{
        let pagination = $crate::Pagination::new($page, $per_page);
        $crate::ResourceCollection::<$resource>::paginated(
            $models.into_iter().map(<$resource as $crate::Resource>::from_model).collect(),
            pagination,
            $total,
        )
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Resource, ResourceCollection};
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestResource {
        id: i32,
        name: String,
    }

    struct TestModel {
        id: i32,
        name: String,
    }

    impl_resource!(TestResource, TestModel, |model| {
        TestResource {
            id: model.id,
            name: model.name,
        }
    });

    #[test]
    fn test_impl_resource_macro() {
        let model = TestModel {
            id: 1,
            name: "Test".to_string(),
        };
        let resource = TestResource::from_model(model);
        assert_eq!(resource.id, 1);
        assert_eq!(resource.name, "Test");
    }
}
