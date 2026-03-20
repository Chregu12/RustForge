//! Deployment tests for rf-pagination

#[cfg(test)]
mod tests {
    use rf_pagination::{
        Paginator, PaginationLinks, PaginatedResponse,
        CursorPaginator,
    };

    // ── Paginator ────────────────────────────────────────────────

    #[test]
    fn paginator_basic() {
        let p = Paginator::new(100, 10, 1).expect("valid");
        assert_eq!(p.offset(), 0);
        assert_eq!(p.limit(), 10);
        assert!(p.has_next());
        assert!(!p.has_prev());
        assert_eq!(p.next_page(), Some(2));
        assert_eq!(p.prev_page(), None);
        assert_eq!(p.from(), 1);
        assert_eq!(p.to(), 10);
    }

    #[test]
    fn paginator_middle_page() {
        let p = Paginator::new(100, 10, 5).expect("valid");
        assert_eq!(p.offset(), 40);
        assert!(p.has_next());
        assert!(p.has_prev());
        assert_eq!(p.next_page(), Some(6));
        assert_eq!(p.prev_page(), Some(4));
    }

    #[test]
    fn paginator_last_page() {
        let p = Paginator::new(25, 10, 3).expect("valid");
        assert!(!p.has_next());
        assert!(p.has_prev());
        assert_eq!(p.next_page(), None);
    }

    #[test]
    fn paginator_invalid_page() {
        assert!(Paginator::new(100, 10, 0).is_err());
        assert!(Paginator::new(100, 10, -1).is_err());
    }

    #[test]
    fn paginator_invalid_per_page() {
        assert!(Paginator::new(100, 0, 1).is_err());
        assert!(Paginator::new(100, -5, 1).is_err());
    }

    // ── PaginationLinks ──────────────────────────────────────────

    #[test]
    fn pagination_links() {
        let p = Paginator::new(50, 10, 3).expect("valid");
        let links = PaginationLinks::new("/api/users", &p);
        assert!(links.first.is_some());
        assert!(links.last.is_some());
        assert!(links.prev.is_some());
        assert!(links.next.is_some());
    }

    #[test]
    fn pagination_links_first_page() {
        let p = Paginator::new(50, 10, 1).expect("valid");
        let links = PaginationLinks::new("/api/users", &p);
        assert!(links.prev.is_none());
        assert!(links.next.is_some());
    }

    // ── PaginatedResponse ────────────────────────────────────────

    #[test]
    fn paginated_response() {
        let p = Paginator::new(3, 10, 1).expect("valid");
        let response = PaginatedResponse::new(
            vec!["item1", "item2", "item3"],
            p,
            Some("/api/items"),
        );
        assert_eq!(response.data.len(), 3);
        assert_eq!(response.meta.total, 3);
        assert_eq!(response.meta.current_page, 1);
    }

    // ── CursorPaginator ──────────────────────────────────────────

    #[test]
    fn cursor_paginator() {
        let p = CursorPaginator::new(20).expect("valid");
        assert_eq!(p.limit(), 20);
    }

    #[test]
    fn cursor_paginator_with_cursor() {
        let p = CursorPaginator::new(10)
            .expect("valid")
            .after("cursor_abc".into());
        let _ = p;
    }

    #[test]
    fn cursor_paginator_invalid() {
        assert!(CursorPaginator::new(0).is_err());
        assert!(CursorPaginator::new(-1).is_err());
    }
}
