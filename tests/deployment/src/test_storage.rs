//! Deployment tests for rf-storage

#[cfg(test)]
mod tests {
    use rf_storage::{Storage, MemoryStorage, StorageManager, LocalStorage};
    use std::sync::Arc;

    // ── MemoryStorage ────────────────────────────────────────────

    #[tokio::test]
    async fn memory_storage_put_get() {
        let storage = MemoryStorage::new();
        storage.put("test.txt", b"Hello, World!".to_vec()).await.expect("put");
        let content = storage.get("test.txt").await.expect("get");
        assert_eq!(content, b"Hello, World!");
    }

    #[tokio::test]
    async fn memory_storage_exists() {
        let storage = MemoryStorage::new();
        assert!(!storage.exists("missing.txt").await.expect("exists"));
        storage.put("file.txt", b"data".to_vec()).await.expect("put");
        assert!(storage.exists("file.txt").await.expect("exists"));
    }

    #[tokio::test]
    async fn memory_storage_delete() {
        let storage = MemoryStorage::new();
        storage.put("delete_me.txt", b"data".to_vec()).await.expect("put");
        storage.delete("delete_me.txt").await.expect("delete");
        assert!(!storage.exists("delete_me.txt").await.expect("exists"));
    }

    #[tokio::test]
    async fn memory_storage_size() {
        let storage = MemoryStorage::new();
        let data = b"12345".to_vec();
        storage.put("sized.txt", data).await.expect("put");
        let size = storage.size("sized.txt").await.expect("size");
        assert_eq!(size, 5);
    }

    #[tokio::test]
    async fn memory_storage_list() {
        let storage = MemoryStorage::new();
        storage.put("dir/a.txt", b"a".to_vec()).await.expect("put");
        storage.put("dir/b.txt", b"b".to_vec()).await.expect("put");
        storage.put("other/c.txt", b"c".to_vec()).await.expect("put");

        let files = storage.list("dir").await.expect("list");
        assert!(files.len() >= 2);
    }

    #[tokio::test]
    async fn memory_storage_copy() {
        let storage = MemoryStorage::new();
        storage.put("original.txt", b"content".to_vec()).await.expect("put");
        storage.copy("original.txt", "copy.txt").await.expect("copy");
        let content = storage.get("copy.txt").await.expect("get");
        assert_eq!(content, b"content");
    }

    #[tokio::test]
    async fn memory_storage_move() {
        let storage = MemoryStorage::new();
        storage.put("source.txt", b"data".to_vec()).await.expect("put");
        storage.move_file("source.txt", "dest.txt").await.expect("move");
        assert!(!storage.exists("source.txt").await.expect("exists"));
        assert!(storage.exists("dest.txt").await.expect("exists"));
    }

    #[tokio::test]
    async fn memory_storage_url() {
        let storage = MemoryStorage::with_url("https://cdn.example.com");
        let url = storage.url("images/photo.jpg");
        assert!(url.contains("images/photo.jpg"));
    }

    #[test]
    fn memory_storage_helper_methods() {
        let storage = MemoryStorage::new();
        assert_eq!(storage.count(), 0);
        storage.clear();
    }

    // ── StorageManager ───────────────────────────────────────────

    #[test]
    fn storage_manager_add_disk() {
        let mut manager = StorageManager::new();
        manager.add_disk("memory", Arc::new(MemoryStorage::new()));
        assert!(manager.has_disk("memory"));
        assert!(!manager.has_disk("nonexistent"));
    }

    #[test]
    fn storage_manager_disk_names() {
        let mut manager = StorageManager::new();
        manager.add_disk("local", Arc::new(MemoryStorage::new()));
        manager.add_disk("s3", Arc::new(MemoryStorage::new()));
        let names = manager.disk_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn storage_manager_default_disk() {
        let mut manager = StorageManager::new();
        manager.add_disk("main", Arc::new(MemoryStorage::new()));
        manager.set_default("main");
        assert_eq!(manager.default_disk_name(), Some("main"));
        assert!(manager.disk_default().is_ok());
    }

    #[test]
    fn storage_manager_remove_disk() {
        let mut manager = StorageManager::new();
        manager.add_disk("temp", Arc::new(MemoryStorage::new()));
        let removed = manager.remove_disk("temp");
        assert!(removed.is_some());
        assert!(!manager.has_disk("temp"));
    }

    // ── LocalStorage ─────────────────────────────────────────────

    #[tokio::test]
    async fn local_storage_in_temp() {
        let dir = std::env::temp_dir().join("rf_test_storage");
        std::fs::create_dir_all(&dir).ok();
        let storage = LocalStorage::new(
            dir.to_str().unwrap(),
            "http://localhost:8080/storage",
        ).await.expect("create");

        Storage::put(&storage, "test.txt", b"Hello".to_vec()).await.expect("put");
        let content: Vec<u8> = Storage::get(&storage, "test.txt").await.expect("get");
        assert_eq!(content, b"Hello");

        Storage::delete(&storage, "test.txt").await.expect("delete");
        std::fs::remove_dir_all(&dir).ok();
    }
}
