//! Facade macros

/// Create a simple facade accessor
#[macro_export]
macro_rules! create_facade {
    (
        $(#[$meta:meta])*
        $name:ident => $type:ty
    ) => {
        $(#[$meta])*
        #[allow(non_snake_case)]
        pub fn $name() -> &'static $type {
            static INSTANCE: once_cell::sync::Lazy<$type> = once_cell::sync::Lazy::new(|| {
                <$type>::default()
            });
            &INSTANCE
        }
    };
}

/// Define a facade with custom initialization
#[macro_export]
macro_rules! define_facade {
    (
        $(#[$meta:meta])*
        $name:ident => $type:ty, init: $init:expr
    ) => {
        $(#[$meta])*
        #[allow(non_snake_case)]
        pub fn $name() -> &'static $type {
            static INSTANCE: once_cell::sync::Lazy<$type> = once_cell::sync::Lazy::new(|| {
                $init
            });
            &INSTANCE
        }
    };
}

#[cfg(test)]
mod tests {
    #[derive(Default)]
    struct TestService {
        name: String,
    }

    impl TestService {
        fn get_name(&self) -> &str {
            &self.name
        }
    }

    create_facade!(TestFacade => TestService);

    #[test]
    fn test_create_facade() {
        let service = TestFacade();
        assert_eq!(service.name, "");
    }

    define_facade!(
        CustomFacade => TestService,
        init: TestService {
            name: "custom".to_string()
        }
    );

    #[test]
    fn test_define_facade() {
        let service = CustomFacade();
        assert_eq!(service.get_name(), "custom");
    }
}
