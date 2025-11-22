use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr, TransactionTrait};
use std::future::Future;

/// Transaction helper for Laravel-style database transactions
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::transaction::Transaction;
///
/// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// // Automatic rollback on error
/// let result = Transaction::run(&db, |tx| async move {
///     // Create user
///     let user = User::create(tx, user_data).await?;
///
///     // Create profile
///     let profile = Profile::create(tx, profile_data).await?;
///
///     // If any error occurs, transaction will rollback automatically
///     Ok((user, profile))
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub struct Transaction;

impl Transaction {
    /// Run a closure in a database transaction
    ///
    /// The transaction will automatically commit if the closure returns Ok,
    /// or rollback if it returns Err or panics.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// Transaction::run(&db, |tx| async move {
    ///     User::create(tx, user).await?;
    ///     Post::create(tx, post).await?;
    ///     Ok(())
    /// }).await?;
    /// ```
    pub async fn run<F, T, Fut>(db: &DatabaseConnection, f: F) -> Result<T, DbErr>
    where
        F: FnOnce(&DatabaseTransaction) -> Fut,
        Fut: Future<Output = Result<T, DbErr>>,
    {
        let txn = db.begin().await?;

        match f(&txn).await {
            Ok(result) => {
                txn.commit().await?;
                Ok(result)
            }
            Err(e) => {
                txn.rollback().await?;
                Err(e)
            }
        }
    }

    /// Begin a new transaction manually
    ///
    /// You are responsible for calling commit() or rollback()
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let tx = Transaction::begin(&db).await?;
    ///
    /// match User::create(&tx, user).await {
    ///     Ok(_) => tx.commit().await?,
    ///     Err(e) => {
    ///         tx.rollback().await?;
    ///         return Err(e);
    ///     }
    /// }
    /// ```
    pub async fn begin(db: &DatabaseConnection) -> Result<DatabaseTransaction, DbErr> {
        db.begin().await
    }
}

/// Extension trait for DatabaseConnection to add Laravel-style transaction method
pub trait TransactionExt {
    /// Run a closure in a transaction (Laravel style)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// db.transaction(|tx| async move {
    ///     User::create(tx, user).await?;
    ///     Ok(())
    /// }).await?;
    /// ```
    async fn transaction<F, T, Fut>(&self, f: F) -> Result<T, DbErr>
    where
        F: FnOnce(&DatabaseTransaction) -> Fut,
        Fut: Future<Output = Result<T, DbErr>>;
}

impl TransactionExt for DatabaseConnection {
    async fn transaction<F, T, Fut>(&self, f: F) -> Result<T, DbErr>
    where
        F: FnOnce(&DatabaseTransaction) -> Fut,
        Fut: Future<Output = Result<T, DbErr>>,
    {
        Transaction::run(self, f).await
    }
}

/// Savepoint support for nested transactions
///
/// # Example
///
/// ```rust,no_run
/// db.transaction(|tx| async move {
///     User::create(tx, user).await?;
///
///     // Nested transaction with savepoint
///     let savepoint = Savepoint::create(tx, "my_savepoint").await?;
///
///     match Post::create(tx, post).await {
///         Ok(_) => savepoint.release().await?,
///         Err(_) => savepoint.rollback().await?,
///     }
///
///     Ok(())
/// }).await?;
/// ```
pub struct Savepoint<'a> {
    tx: &'a DatabaseTransaction,
    name: String,
}

impl<'a> Savepoint<'a> {
    /// Create a new savepoint
    pub async fn create(tx: &'a DatabaseTransaction, name: &str) -> Result<Self, DbErr> {
        tx.execute_unprepared(&format!("SAVEPOINT {}", name))
            .await?;

        Ok(Self {
            tx,
            name: name.to_string(),
        })
    }

    /// Release the savepoint (commit nested transaction)
    pub async fn release(self) -> Result<(), DbErr> {
        self.tx
            .execute_unprepared(&format!("RELEASE SAVEPOINT {}", self.name))
            .await?;
        Ok(())
    }

    /// Rollback to the savepoint
    pub async fn rollback(&self) -> Result<(), DbErr> {
        self.tx
            .execute_unprepared(&format!("ROLLBACK TO SAVEPOINT {}", self.name))
            .await?;
        Ok(())
    }
}

/// Transaction isolation levels
#[derive(Debug, Clone, Copy)]
pub enum IsolationLevel {
    /// Read uncommitted (lowest isolation)
    ReadUncommitted,
    /// Read committed (default in most databases)
    ReadCommitted,
    /// Repeatable read
    RepeatableRead,
    /// Serializable (highest isolation)
    Serializable,
}

impl IsolationLevel {
    /// Get the SQL string for this isolation level
    pub fn to_sql(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED",
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

/// Extension for setting transaction isolation level
pub trait IsolationLevelExt {
    /// Set the isolation level for the next transaction
    async fn set_isolation_level(&self, level: IsolationLevel) -> Result<(), DbErr>;
}

impl IsolationLevelExt for DatabaseConnection {
    async fn set_isolation_level(&self, level: IsolationLevel) -> Result<(), DbErr> {
        self.execute_unprepared(&format!(
            "SET TRANSACTION ISOLATION LEVEL {}",
            level.to_sql()
        ))
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_sql() {
        assert_eq!(IsolationLevel::ReadCommitted.to_sql(), "READ COMMITTED");
        assert_eq!(IsolationLevel::Serializable.to_sql(), "SERIALIZABLE");
    }
}
