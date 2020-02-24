use crate::email::Email;

use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::storage;

pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl From<i32> for LogLevel {
    fn from(l: i32) -> Self {
        match l {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warning,
            3 => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

/// Single address row in DB
#[derive(Clone)]
pub struct Address {
    pub address: String,
    pub user_id: i32,
    pub max_email_size: i32,
    pub quota: i32,
    pub received: i32,
    pub storage_token: String,
    pub storage_backend: storage::Backend,
    pub storage_path: String,
    pub last_renewal_time: DateTime<Utc>,
}

/// Abstraction over sqlx DB client for Vaulty DB
pub struct Client<'a> {
    pub db: &'a mut sqlx::PgPool,
    pub user_table: String,
    pub address_table: String,
    pub email_table: String,
    pub log_table: String,
}

impl<'a> Client<'a> {
    pub fn new(db: &'a mut sqlx::PgPool) -> Self {
        Client {
            db: db,
            user_table: "users".to_string(),
            address_table: "addresses".to_string(),
            email_table: "emails".to_string(),
            log_table: "logs".to_string(),
        }
    }

    /// Convert a recipient email to a user ID
    pub async fn get_user_id(
        &mut self,
        recipient: &str,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let query = format!(
            "SELECT user_id FROM {} WHERE address = $1",
            &self.address_table
        );

        let row = sqlx::query(&query)
            .bind(recipient)
            .fetch_one(self.db)
            .await?;

        let user_id: i32 = row.get("user_id");

        Ok(user_id)
    }

    /// Convert a list of recipient emails into address info.
    ///
    /// This function will only return info for the **first** valid recipient
    /// email in the provided list.
    pub async fn get_address(
        &mut self,
        recipients: &Vec<&str>,
    ) -> Result<Option<Address>, Box<dyn std::error::Error>> {
        // Build a SQL list of values to check against
        // NOTE: This may need to be sanitizied
        let address_list = recipients
            .iter()
            .map(|r| format!("'{}'", r))
            .collect::<Vec<String>>()
            .join(", ");

        let query = format!(
            "SELECT * FROM {} WHERE address IN ({})",
            &self.address_table, &address_list
        );

        let row = sqlx::query(&query).fetch_optional(self.db).await?;

        if let Some(data) = row {
            let address = Address {
                address: data.get("address"),
                user_id: data.get("user_id"),
                max_email_size: data.get("max_email_size"),
                quota: data.get("quota"),
                received: data.get("received"),
                storage_token: data.get("storage_token"),
                storage_backend: data.get::<String, &str>("storage_backend").into(),
                storage_path: data.get("storage_path"),
                last_renewal_time: data.get("last_renewal_time"),
            };

            Ok(Some(address))
        } else {
            // If no rows returned, none of the recipients are valid
            Ok(None)
        }
    }

    /// Update address mail received count
    pub async fn update_address_received_count(
        &mut self,
        address: &Address,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // For now, just increment the received count
        let query = format!(
            "
            UPDATE {}
            SET received = received + 1
            WHERE address = $1",
            &self.address_table
        );

        let _num_rows = sqlx::query(&query)
            .bind(&address.address)
            .execute(self.db)
            .await?;

        Ok(())
    }

    /// Log a message to the logs table
    ///
    /// If this fails, we just log an error internally and proceed.
    ///
    /// `email_id` is optional since we may insert logs before inserting an
    /// email (e.g., rejected email).
    pub async fn log(&mut self, msg: &str, email_id: Option<&uuid::Uuid>, log_level: LogLevel) {
        let query = format!(
            "
            INSERT INTO {0}
            (email_id, msg, log_level) VALUES
            ($1, $2, $3)",
            &self.log_table
        );

        let num_rows = sqlx::query(&query)
            .bind(email_id)
            .bind(msg)
            .bind(log_level as i32)
            .execute(self.db)
            .await;

        if let Err(e) = num_rows {
            log::error!("Failed to log to DB: {}", e.to_string());
        }
    }

    /// Insert an email into DB
    /// Status and error message must be updated later
    pub async fn insert_email(&mut self, email: &Email) -> Result<(), Box<dyn std::error::Error>> {
        let email_id = &email.uuid;
        let num_attachments = email.num_attachments.unwrap_or(0);

        // Recipient list will have been filtered down at this point
        let recipient = &email.recipients[0];

        let total_size = email.size;
        let creation_time: DateTime<Utc> = Utc::now();

        let query = format!("
            INSERT INTO {0} (user_id, address_id, email_id, num_attachments, total_size, message_id, creation_time) VALUES
            ((SELECT user_id FROM {1} WHERE address = $1),
             (SELECT id FROM {1} WHERE address = $1), $2, $3, $4, $5, $6)",
            &self.email_table, &self.address_table
        );

        let _num_rows = sqlx::query(&query)
            .bind(recipient)
            .bind(email_id)
            .bind(num_attachments as i32)
            .bind(total_size as i32)
            .bind(email.message_id.as_ref())
            .bind(creation_time)
            .execute(self.db)
            .await?;

        Ok(())
    }

    /// Update email status (success or failure)
    /// We do not really care if this operation fails (best-effort)
    pub async fn update_email(&mut self, email: &Email, status: bool, msg: Option<&str>) {
        let email_id = &email.uuid;

        let query = format!(
            "
            UPDATE {}
            SET status = $1, error_msg = $2
            WHERE email_id = $3",
            &self.email_table
        );

        let num_rows = sqlx::query(&query)
            .bind(status)
            .bind(msg)
            .bind(email_id)
            .execute(self.db)
            .await;

        if let Err(e) = num_rows {
            log::error!("Failed to update email: {}", e.to_string());
        }
    }
}
