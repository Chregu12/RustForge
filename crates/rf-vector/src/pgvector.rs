//! SQL helpers for Postgres + the [pgvector](https://github.com/pgvector/pgvector)
//! extension.
//!
//! These are **pure string helpers** — this crate pulls in no database driver.
//! They are designed to be dropped into rf-orm query builders via `where_raw`
//! and `order_by_raw`:
//!
//! ```rust
//! use rf_vector::{Vector, DistanceMetric};
//! use rf_vector::pgvector::order_by_nearest;
//!
//! let q = Vector::new(vec![0.1, 0.2, 0.3]);
//! let fragment = order_by_nearest("embedding", &q, DistanceMetric::Cosine);
//! // query.order_by_raw(&fragment) in rf-orm
//! assert_eq!(fragment, "embedding <=> '[0.1,0.2,0.3]'");
//! ```

use crate::vector::{DistanceMetric, Vector};

/// Format a vector as a pgvector literal, e.g. `'[0.1,0.2,0.3]'`.
///
/// Floats are printed compactly (no padding / trailing-zero bloat) using their
/// shortest round-trippable representation.
pub fn to_literal(v: &Vector) -> String {
    let mut out = String::from("'[");
    for (i, x) in v.as_slice().iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format_float(*x));
    }
    out.push_str("]'");
    out
}

fn format_float(x: f32) -> String {
    // `{}` on f32 already yields the shortest representation that round-trips
    // (e.g. 0.1 -> "0.1", 2.0 -> "2").
    format!("{x}")
}

/// The pgvector distance operator for a metric.
///
/// - `Cosine` → `<=>` (cosine distance)
/// - `Euclidean` → `<->` (L2 distance)
/// - `DotProduct` → `<#>` (negative inner product)
pub fn operator(metric: DistanceMetric) -> &'static str {
    match metric {
        DistanceMetric::Cosine => "<=>",
        DistanceMetric::Euclidean => "<->",
        DistanceMetric::DotProduct => "<#>",
    }
}

/// Build an `ORDER BY` fragment that ranks rows nearest to `query`.
///
/// Suitable for rf-orm's `order_by_raw`, e.g.
/// `order_by_raw(&order_by_nearest("embedding", &q, DistanceMetric::Cosine))`.
///
/// Produces, for cosine: `embedding <=> '[...]'`.
pub fn order_by_nearest(column: &str, query: &Vector, metric: DistanceMetric) -> String {
    format!("{column} {} {}", operator(metric), to_literal(query))
}

/// Build a complete illustrative nearest-neighbour query.
///
/// `SELECT * FROM {table} ORDER BY {column} <op> '[...]' LIMIT {limit}`.
pub fn nearest_neighbor_sql(
    table: &str,
    column: &str,
    query: &Vector,
    metric: DistanceMetric,
    limit: usize,
) -> String {
    format!(
        "SELECT * FROM {table} ORDER BY {} LIMIT {limit}",
        order_by_nearest(column, query, metric)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_literal_exact() {
        let v = Vector::new(vec![0.1, 0.2, 0.3]);
        assert_eq!(to_literal(&v), "'[0.1,0.2,0.3]'");
    }

    #[test]
    fn to_literal_compact_whole_numbers() {
        let v = Vector::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(to_literal(&v), "'[1,2,3]'");
    }

    #[test]
    fn operator_exact() {
        assert_eq!(operator(DistanceMetric::Cosine), "<=>");
        assert_eq!(operator(DistanceMetric::Euclidean), "<->");
        assert_eq!(operator(DistanceMetric::DotProduct), "<#>");
    }

    #[test]
    fn order_by_nearest_exact() {
        let v = Vector::new(vec![1.0, 2.0]);
        assert_eq!(
            order_by_nearest("embedding", &v, DistanceMetric::Cosine),
            "embedding <=> '[1,2]'"
        );
    }

    #[test]
    fn nearest_neighbor_sql_exact() {
        let v = Vector::new(vec![1.0, 2.0]);
        assert_eq!(
            nearest_neighbor_sql("docs", "embedding", &v, DistanceMetric::Euclidean, 5),
            "SELECT * FROM docs ORDER BY embedding <-> '[1,2]' LIMIT 5"
        );
    }
}

#[cfg(test)]
mod adversarial {
    use super::*;

    #[test]
    fn to_literal_negative_and_mixed() {
        let v = Vector::new(vec![-0.1, 2.0, -3.5, 0.0]);
        assert_eq!(to_literal(&v), "'[-0.1,2,-3.5,0]'");
    }

    #[test]
    fn to_literal_single_and_empty() {
        assert_eq!(to_literal(&Vector::new(vec![1.5])), "'[1.5]'");
        // empty vector still produces well-formed brackets
        assert_eq!(to_literal(&Vector::new(vec![])), "'[]'");
    }

    #[test]
    fn to_literal_roundtrips() {
        // Every emitted number must parse back to the exact f32.
        let vals = vec![0.1f32, 0.2, -0.333_333, 1e-7, 12345.678, -0.0, 100.0];
        let v = Vector::new(vals.clone());
        let lit = to_literal(&v);
        let inner = lit
            .trim_start_matches("'[")
            .trim_end_matches("]'");
        let parsed: Vec<f32> = inner.split(',').map(|s| s.parse().unwrap()).collect();
        for (orig, got) in vals.iter().zip(parsed.iter()) {
            assert_eq!(orig.to_bits(), got.to_bits(), "{orig} != {got}");
        }
    }

    #[test]
    fn literal_contains_only_safe_chars() {
        // The vector portion must contain no quotes/semicolons that could break
        // out of the SQL string literal — only digits, sign, dot, e, comma.
        let v = Vector::new(vec![-1.5e3, 0.000_123, 42.0]);
        let lit = to_literal(&v);
        let inner = lit.trim_start_matches("'[").trim_end_matches("]'");
        assert!(
            inner
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '-' | '.' | ',' | 'e' | 'E' | '+')),
            "unexpected char in literal: {inner}"
        );
        // exactly two surrounding single quotes, none in the middle
        assert_eq!(lit.matches('\'').count(), 2);
    }

    #[test]
    fn non_finite_is_not_silently_quote_breaking() {
        // NaN/inf can't appear in a valid pgvector literal, but they must at
        // least not inject quotes. Document Rust's formatting: NaN/inf/-inf.
        let v = Vector::new(vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY]);
        let lit = to_literal(&v);
        assert_eq!(lit.matches('\'').count(), 2);
        // These are NOT valid pgvector and would error at the DB — caveat noted.
        assert!(lit.contains("NaN") || lit.contains("nan"));
        assert!(lit.contains("inf"));
    }

    #[test]
    fn operator_mapping_complete() {
        assert_eq!(operator(DistanceMetric::Cosine), "<=>");
        assert_eq!(operator(DistanceMetric::Euclidean), "<->");
        assert_eq!(operator(DistanceMetric::DotProduct), "<#>");
    }

    #[test]
    fn order_by_nearest_all_metrics() {
        let v = Vector::new(vec![0.1, 0.2, 0.3]);
        assert_eq!(
            order_by_nearest("embedding", &v, DistanceMetric::Cosine),
            "embedding <=> '[0.1,0.2,0.3]'"
        );
        assert_eq!(
            order_by_nearest("embedding", &v, DistanceMetric::Euclidean),
            "embedding <-> '[0.1,0.2,0.3]'"
        );
        assert_eq!(
            order_by_nearest("embedding", &v, DistanceMetric::DotProduct),
            "embedding <#> '[0.1,0.2,0.3]'"
        );
    }

    #[test]
    fn nearest_neighbor_sql_all_parts() {
        let v = Vector::new(vec![1.0, -2.0]);
        let sql = nearest_neighbor_sql("docs", "embedding", &v, DistanceMetric::Cosine, 10);
        assert_eq!(
            sql,
            "SELECT * FROM docs ORDER BY embedding <=> '[1,-2]' LIMIT 10"
        );
    }

    #[test]
    fn identifiers_interpolated_verbatim_caveat() {
        // CAVEAT: column/table are interpolated unescaped (expected for SQL
        // identifiers — callers must pass trusted names). Documenting the
        // behavior so it's an explicit, tested contract.
        let v = Vector::new(vec![1.0]);
        let sql = nearest_neighbor_sql("t; DROP TABLE x", "c", &v, DistanceMetric::Cosine, 1);
        assert!(sql.contains("t; DROP TABLE x"));
    }
}
