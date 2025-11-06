use crate::context::GraphQLContext;
use crate::error::{database_error, not_found, validation_error};
use crate::types::account::{Account, AccountEntity, AccountColumn, AccountActiveModel, AccountInput, UpdateAccountInput};
use async_graphql::{Context, Object, Result};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

#[derive(Default)]
pub struct AccountQuery;

#[Object]
impl AccountQuery {
    /// Get an account by ID
    async fn account(&self, ctx: &Context<'_>, id: i64) -> Result<Account> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let account = AccountEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("Account with id {} not found", id)))?;

        Ok(account.into())
    }

    /// Get all accounts with optional pagination
    async fn accounts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 0)] offset: u64,
        #[graphql(default = 10)] limit: u64,
    ) -> Result<Vec<Account>> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let accounts = AccountEntity::find()
            .order_by_asc(AccountColumn::Id)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await
            .map_err(database_error)?;

        Ok(accounts.into_iter().map(|a| a.into()).collect())
    }

    /// Count total accounts
    async fn accounts_count(&self, ctx: &Context<'_>) -> Result<u64> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        AccountEntity::find()
            .count(db)
            .await
            .map_err(database_error)
    }

    /// Get account by email
    async fn account_by_email(&self, ctx: &Context<'_>, email: String) -> Result<Account> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let account = AccountEntity::find()
            .filter(AccountColumn::Email.eq(email.clone()))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("Account with email {} not found", email)))?;

        Ok(account.into())
    }

    /// Get active accounts only
    async fn active_accounts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10)] limit: u64,
    ) -> Result<Vec<Account>> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let accounts = AccountEntity::find()
            .filter(AccountColumn::Active.eq(true))
            .limit(limit)
            .all(db)
            .await
            .map_err(database_error)?;

        Ok(accounts.into_iter().map(|a| a.into()).collect())
    }

    /// Get accounts by role
    async fn accounts_by_role(
        &self,
        ctx: &Context<'_>,
        role: String,
        #[graphql(default = 10)] limit: u64,
    ) -> Result<Vec<Account>> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let accounts = AccountEntity::find()
            .filter(AccountColumn::Role.eq(role))
            .limit(limit)
            .all(db)
            .await
            .map_err(database_error)?;

        Ok(accounts.into_iter().map(|a| a.into()).collect())
    }
}

#[derive(Default)]
pub struct AccountMutation;

#[Object]
impl AccountMutation {
    /// Create a new account
    async fn create_account(&self, ctx: &Context<'_>, input: AccountInput) -> Result<Account> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        // Validate email format
        if !input.email.contains('@') {
            return Err(validation_error("Invalid email format"));
        }

        // Check if email already exists
        let existing = AccountEntity::find()
            .filter(AccountColumn::Email.eq(&input.email))
            .one(db)
            .await
            .map_err(database_error)?;

        if existing.is_some() {
            return Err(validation_error(format!(
                "Account with email '{}' already exists",
                input.email
            )));
        }

        let now = Utc::now().naive_utc();
        let account = AccountActiveModel {
            email: ActiveValue::Set(input.email),
            name: ActiveValue::Set(input.name),
            role: ActiveValue::Set(input.role),
            active: ActiveValue::Set(input.active.unwrap_or(true)),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };

        let result = account.insert(db).await.map_err(database_error)?;
        Ok(result.into())
    }

    /// Update an existing account
    async fn update_account(
        &self,
        ctx: &Context<'_>,
        id: i64,
        input: UpdateAccountInput,
    ) -> Result<Account> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        // Find existing account
        let account = AccountEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("Account with id {} not found", id)))?;

        let mut active: AccountActiveModel = account.into();

        // Update fields if provided
        if let Some(email) = input.email {
            // Validate email format
            if !email.contains('@') {
                return Err(validation_error("Invalid email format"));
            }

            // Check if new email already exists
            let existing = AccountEntity::find()
                .filter(AccountColumn::Email.eq(&email))
                .filter(AccountColumn::Id.ne(id))
                .one(db)
                .await
                .map_err(database_error)?;

            if existing.is_some() {
                return Err(validation_error(format!(
                    "Account with email '{}' already exists",
                    email
                )));
            }
            active.email = ActiveValue::Set(email);
        }
        if let Some(name) = input.name {
            active.name = ActiveValue::Set(name);
        }
        if let Some(role) = input.role {
            active.role = ActiveValue::Set(role);
        }
        if let Some(active_flag) = input.active {
            active.active = ActiveValue::Set(active_flag);
        }

        active.updated_at = ActiveValue::Set(Utc::now().naive_utc());

        let result = active.update(db).await.map_err(database_error)?;
        Ok(result.into())
    }

    /// Delete an account
    async fn delete_account(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let account = AccountEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("Account with id {} not found", id)))?;

        let active: AccountActiveModel = account.into();
        active.delete(db).await.map_err(database_error)?;

        Ok(true)
    }
}
