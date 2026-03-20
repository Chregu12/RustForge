//! Deployment tests for rf-collections

#[cfg(test)]
mod tests {
    use rf_collections::{collect, collect_lazy, Collection};

    // ── Basic Operations ─────────────────────────────────────────

    #[test]
    fn collection_new_and_count() {
        let c = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(c.count(), 5);
        assert!(!c.is_empty());
    }

    #[test]
    fn collection_empty() {
        let c: Collection<i32> = Collection::new(vec![]);
        assert!(c.is_empty());
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn collect_helper() {
        let c = collect(vec!["a", "b", "c"]);
        assert_eq!(c.count(), 3);
    }

    #[test]
    fn collection_first_last() {
        let c = collect(vec![10, 20, 30]);
        assert_eq!(c.first(), Some(&10));
        assert_eq!(c.last(), Some(&30));
    }

    // ── Transformations ──────────────────────────────────────────

    #[test]
    fn collection_map() {
        let c = collect(vec![1, 2, 3]).map(|x| x * 2);
        assert_eq!(c.all(), &[2, 4, 6]);
    }

    #[test]
    fn collection_filter() {
        let c = collect(vec![1, 2, 3, 4, 5]).filter(|x| x % 2 == 0);
        assert_eq!(c.all(), &[2, 4]);
    }

    #[test]
    fn collection_take_skip() {
        let c = collect(vec![1, 2, 3, 4, 5]);
        assert_eq!(c.clone().take(3).all(), &[1, 2, 3]);
        assert_eq!(c.skip(3).all(), &[4, 5]);
    }

    #[test]
    fn collection_chunk() {
        let chunks = collect(vec![1, 2, 3, 4, 5]).chunk(2);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].all(), &[1, 2]);
        assert_eq!(chunks[1].all(), &[3, 4]);
        assert_eq!(chunks[2].all(), &[5]);
    }

    #[test]
    fn collection_reverse() {
        let c = collect(vec![1, 2, 3]).reverse();
        assert_eq!(c.all(), &[3, 2, 1]);
    }

    #[test]
    fn collection_reduce() {
        let sum = collect(vec![1, 2, 3, 4]).reduce(0, |acc, x| acc + x);
        assert_eq!(sum, 10);
    }

    #[test]
    fn collection_pluck() {
        let c = collect(vec![(1, "a"), (2, "b"), (3, "c")]);
        let keys = c.pluck(|(k, _)| *k);
        assert_eq!(keys.count(), 3);
        assert_eq!(keys.first(), Some(&1));
    }

    // ── Searching & Checking ─────────────────────────────────────

    #[test]
    fn collection_find() {
        let c = collect(vec![10, 20, 30]);
        assert_eq!(c.find(|x| *x > 15), Some(&20));
    }

    #[test]
    fn collection_contains() {
        let c = collect(vec![1, 2, 3]);
        assert!(c.contains(&2));
        assert!(!c.contains(&5));
    }

    #[test]
    fn collection_any_all() {
        let c = collect(vec![2, 4, 6]);
        assert!(c.any(|x| *x == 4));
        assert!(c.all_match(|x| *x % 2 == 0));
    }

    // ── Sorting ──────────────────────────────────────────────────

    #[test]
    fn collection_sort() {
        let c = collect(vec![3, 1, 4, 1, 5]).sort();
        assert_eq!(c.all(), &[1, 1, 3, 4, 5]);
    }

    #[test]
    fn collection_sort_by() {
        let c = collect(vec!["banana", "apple", "cherry"]).sort_by(|s| s.len());
        assert_eq!(c.first(), Some(&"apple"));
    }

    // ── Unique ───────────────────────────────────────────────────

    #[test]
    fn collection_unique() {
        let c = collect(vec![1, 2, 2, 3, 3, 3]).unique();
        assert_eq!(c.count(), 3);
    }

    // ── Aggregation ──────────────────────────────────────────────

    #[test]
    fn collection_sum() {
        let sum: i32 = collect(vec![1, 2, 3, 4]).sum();
        assert_eq!(sum, 10);
    }

    #[test]
    fn collection_avg() {
        let avg = collect(vec![10.0, 20.0, 30.0]).avg();
        assert_eq!(avg, Some(20.0));
    }

    #[test]
    fn collection_min_max() {
        let c = collect(vec![5, 2, 8, 1, 9]);
        assert_eq!(c.clone().min(), Some(1));
        assert_eq!(c.max(), Some(9));
    }

    #[test]
    fn collection_median() {
        let median = collect(vec![1, 3, 5]).median();
        assert_eq!(median, Some(3.0));
    }

    // ── Stack Operations ─────────────────────────────────────────

    #[test]
    fn collection_push_prepend() {
        let c = collect(vec![2, 3]).prepend(1).push(4);
        assert_eq!(c.all(), &[1, 2, 3, 4]);
    }

    #[test]
    fn collection_pop_shift() {
        let (c, popped) = collect(vec![1, 2, 3]).pop();
        assert_eq!(popped, Some(3));
        assert_eq!(c.count(), 2);

        let (c2, shifted) = collect(vec![1, 2, 3]).shift();
        assert_eq!(shifted, Some(1));
        assert_eq!(c2.count(), 2);
    }

    // ── Partitioning ─────────────────────────────────────────────

    #[test]
    fn collection_partition() {
        let (even, odd) = collect(vec![1, 2, 3, 4, 5]).partition(|x| x % 2 == 0);
        assert_eq!(even.count(), 2);
        assert_eq!(odd.count(), 3);
    }

    // ── Group By ─────────────────────────────────────────────────

    #[test]
    fn collection_group_by() {
        let groups = collect(vec![1, 2, 3, 4, 5, 6]).group_by(|x| x % 2 == 0);
        assert_eq!(groups.len(), 2);
    }

    // ── String Operations ────────────────────────────────────────

    #[test]
    fn collection_join() {
        let joined = collect(vec!["a", "b", "c"]).join(", ");
        assert_eq!(joined, "a, b, c");
    }

    #[test]
    fn collection_implode() {
        let result = collect(vec![1, 2, 3]).implode("-");
        assert_eq!(result, "1-2-3");
    }

    // ── Pagination ───────────────────────────────────────────────

    #[test]
    fn collection_for_page() {
        let page = collect(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).for_page(2, 3);
        assert_eq!(page.all(), &[4, 5, 6]);
    }

    // ── Conditional ──────────────────────────────────────────────

    #[test]
    fn collection_when_unless() {
        let c = collect(vec![1, 2, 3])
            .when(true, |c| c.push(4))
            .when(false, |c| c.push(5))
            .unless(false, |c| c.push(6));
        assert!(c.contains(&4));
        assert!(!c.contains(&5));
        assert!(c.contains(&6));
    }

    // ── Zip & Merge ──────────────────────────────────────────────

    #[test]
    fn collection_zip() {
        let zipped = collect(vec![1, 2, 3]).zip(vec!["a", "b", "c"]);
        assert_eq!(zipped.count(), 3);
        assert_eq!(zipped.first(), Some(&(1, "a")));
    }

    #[test]
    fn collection_merge() {
        let c = collect(vec![1, 2]).merge(collect(vec![3, 4]));
        assert_eq!(c.all(), &[1, 2, 3, 4]);
    }

    // ── Lazy Collection ──────────────────────────────────────────

    #[test]
    fn lazy_collection_basic() {
        let result: Vec<i32> = collect_lazy(0..100)
            .filter(|x| x % 2 == 0)
            .map(|x| x * 2)
            .take(5)
            .collect();
        assert_eq!(result, vec![0, 4, 8, 12, 16]);
    }

    #[test]
    fn lazy_collection_count() {
        let count = collect_lazy(0..50).filter(|x| x % 3 == 0).count();
        assert_eq!(count, 17); // 0,3,6,...,48
    }

    // ── Pipe & Tap ───────────────────────────────────────────────

    #[test]
    fn collection_pipe() {
        let sum = collect(vec![1, 2, 3]).pipe(|c| c.reduce(0, |a, x| a + x));
        assert_eq!(sum, 6);
    }

    #[test]
    fn collection_tap() {
        let mut seen = false;
        let c = collect(vec![1, 2, 3]).tap(|_c| {
            seen = true;
        });
        assert!(seen);
        assert_eq!(c.count(), 3);
    }
}
