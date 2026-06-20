use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr, TransactionTrait};
use std::future::Future;

/// Transaction helper for Laravel-style database transactions
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::transaction::Transaction;
/// # use sea_orm::{DatabaseConnection, ConnectionTrait};
///
/// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// // The closure receives a transaction handle. If it returns `Err`, the
/// // transaction is rolled back automatically; on `Ok` it is committed.
/// Transaction::run(&db, |_tx| async move {
///     // ... run queries against the transaction handle here ...
///     Ok::<(), sea_orm::DbErr>(())
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
    /// # use rf_orm::transaction::Transaction;
    /// # use sea_orm::{DatabaseConnection, ConnectionTrait};
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// Transaction::run(&db, |_tx| async move {
    ///     // ... run queries against the transaction handle here ...
    ///     Ok::<(), sea_orm::DbErr>(())
    /// }).await?;
    /// # Ok(())
    /// # }
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
    /// # use rf_orm::transaction::Transaction;
    /// # use sea_orm::{DatabaseConnection, ConnectionTrait, TransactionTrait};
    /// # async fn example(db: DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    /// let tx = Transaction::begin(&db).await?;
    ///
    /// match tx.execute_unprepared("INSERT INTO users (name) VALUES ('John')").await {
    ///     Ok(_) => tx.commit().await?,
    ///     Err(e) => {
    ///         tx.rollback().await?;
    ///         return Err(e);
    ///     }
    /// }
    /// # Ok(())
    /// # }
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
    /// # use rf_orm::transaction::TransactionExt;
    /// # use sea_orm::{DatabaseConnection, ConnectionTrait};
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// db.transaction(|_tx| async move {
    ///     // ... run queries against the transaction handle here ...
    ///     Ok::<(), sea_orm::DbErr>(())
    /// }).await?;
    /// # Ok(())
    /// # }
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
/// # use rf_orm::transaction::TransactionExt;
/// # use sea_orm::DatabaseConnection;
/// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// db.transaction(|_tx| async move {
///     // Create a savepoint, then run nested work:
///     //   let savepoint = Savepoint::create(tx, "my_savepoint").await?;
///     //   match do_work(tx).await {
///     //       Ok(_)  => savepoint.release().await?,
///     //       Err(_) => savepoint.rollback().await?,
///     //   }
///     Ok::<(), sea_orm::DbErr>(())
/// }).await?;
/// # Ok(())
/// # }
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
