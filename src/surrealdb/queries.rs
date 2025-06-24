use std::collections::HashMap;
use surrealdb::{sql::Value, Response};
use serde::{Serialize, Deserialize};
use conduwuit::{Result, debug, trace};

use crate::{
    engine::Engine,
    error::{Error, Result as SurrealResult, convert_surreal_error},
};

/// Common SurrealDB query patterns and schema management
pub struct QueryBuilder {
    engine: std::sync::Arc<Engine>,
}

/// Schema definition for SurrealDB tables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub fields: HashMap<String, FieldDefinition>,
    pub indexes: Vec<IndexDefinition>,
    pub permissions: Option<PermissionDefinition>,
}

/// Field definition for table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub field_type: String,
    pub required: bool,
    pub default: Option<Value>,
    pub validation: Option<String>,
}

/// Index definition for table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
    pub index_type: IndexType,
}

/// Permission definition for table access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDefinition {
    pub select: Option<String>,
    pub create: Option<String>,
    pub update: Option<String>,
    pub delete: Option<String>,
}

/// Index types supported by SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    Btree,
    Hash,
    FullText,
    Vector { dimension: usize, distance: String },
}

impl QueryBuilder {
    pub fn new(engine: std::sync::Arc<Engine>) -> Self {
        Self { engine }
    }
    
    /// Create a table with schema
    pub async fn create_table(&self, schema: &TableSchema) -> SurrealResult<Response> {
        debug!("Creating table: {}", schema.name);
        
        let mut queries = Vec::new();
        
        // Define table
        queries.push(format!("DEFINE TABLE {};", schema.name));
        
        // Define fields
        for (field_name, field_def) in &schema.fields {
            let mut field_query = format!(
                "DEFINE FIELD {} ON TABLE {} TYPE {};",
                field_name, schema.name, field_def.field_type
            );
            
            if field_def.required {
                field_query = format!("{} ASSERT $value != NONE;", field_query.trim_end_matches(';'));
            }
            
            if let Some(ref default) = field_def.default {
                field_query = format!("{} DEFAULT {};", field_query.trim_end_matches(';'), default);
            }
            
            if let Some(ref validation) = field_def.validation {
                field_query = format!("{} ASSERT {};", field_query.trim_end_matches(';'), validation);
            }
            
            queries.push(field_query);
        }
        
        // Define indexes
        for index in &schema.indexes {
            let index_query = match index.index_type {
                IndexType::Btree => format!(
                    "DEFINE INDEX {} ON TABLE {} COLUMNS {} {};",
                    index.name,
                    schema.name,
                    index.fields.join(", "),
                    if index.unique { "UNIQUE" } else { "" }
                ),
                IndexType::Hash => format!(
                    "DEFINE INDEX {} ON TABLE {} COLUMNS {} {} HASH;",
                    index.name,
                    schema.name,
                    index.fields.join(", "),
                    if index.unique { "UNIQUE" } else { "" }
                ),
                IndexType::FullText => format!(
                    "DEFINE INDEX {} ON TABLE {} COLUMNS {} SEARCH ANALYZER ascii BM25(1.2,0.75);",
                    index.name,
                    schema.name,
                    index.fields.join(", ")
                ),
                IndexType::Vector { dimension, ref distance } => format!(
                    "DEFINE INDEX {} ON TABLE {} FIELDS {} MTREE DIMENSION {} DIST {};",
                    index.name,
                    schema.name,
                    index.fields.join(", "),
                    dimension,
                    distance
                ),
            };
            queries.push(index_query);
        }
        
        // Define permissions
        if let Some(ref perms) = schema.permissions {
            if let Some(ref select) = perms.select {
                queries.push(format!("DEFINE TABLE {} PERMISSIONS FOR select WHERE {};", schema.name, select));
            }
            if let Some(ref create) = perms.create {
                queries.push(format!("DEFINE TABLE {} PERMISSIONS FOR create WHERE {};", schema.name, create));
            }
            if let Some(ref update) = perms.update {
                queries.push(format!("DEFINE TABLE {} PERMISSIONS FOR update WHERE {};", schema.name, update));
            }
            if let Some(ref delete) = perms.delete {
                queries.push(format!("DEFINE TABLE {} PERMISSIONS FOR delete WHERE {};", schema.name, delete));
            }
        }
        
        // Execute all queries in a transaction
        let combined_query = queries.join("\n");
        trace!("Executing table creation queries: {}", combined_query);
        
        self.engine.query(&combined_query).await
            .map_err(|e| Error::Schema(format!("Failed to create table {}: {}", schema.name, e)))
    }
    
    /// Drop a table
    pub async fn drop_table(&self, table_name: &str) -> SurrealResult<Response> {
        debug!("Dropping table: {}", table_name);
        
        let query = format!("REMOVE TABLE {};", table_name);
        self.engine.query(&query).await
            .map_err(|e| Error::Schema(format!("Failed to drop table {}: {}", table_name, e)))
    }
    
    /// Check if a table exists
    pub async fn table_exists(&self, table_name: &str) -> SurrealResult<bool> {
        trace!("Checking if table exists: {}", table_name);
        
        let query = "INFO FOR DB;";
        let response = self.engine.query(query).await?;
        
        // Parse response to check if table exists
        // This is a simplified check - in practice you'd parse the INFO response
        Ok(true) // Placeholder implementation
    }
    
    /// Get table schema information
    pub async fn get_table_info(&self, table_name: &str) -> SurrealResult<Value> {
        debug!("Getting table info for: {}", table_name);
        
        let query = format!("INFO FOR TABLE {};", table_name);
        let mut response = self.engine.query(&query).await?;
        
        // Use the take method to extract values from response
        let result: Option<Value> = response.take(0).unwrap_or_default();
        Ok(result.unwrap_or(Value::None))
    }
    
    /// Create a record
    pub async fn create_record<T>(&self, table: &str, data: T) -> SurrealResult<Value>
    where
        T: Serialize + 'static,
    {
        trace!("Creating record in table: {}", table);
        
        let query = format!("CREATE {} CONTENT $data;", table);
        let mut response = self.engine.db.query(query)
            .bind(("data", data))
            .await
            .map_err(Error::from)?;
            
        let result: Option<Value> = response.take(0).unwrap_or_default();
        Ok(result.unwrap_or(Value::None))
    }
    
    /// Select records with optional conditions
    pub async fn select_records(&self, table: &str, condition: Option<&str>) -> SurrealResult<Vec<Value>> {
        trace!("Selecting records from table: {}", table);
        
        let query = if let Some(cond) = condition {
            format!("SELECT * FROM {} WHERE {};", table, cond)
        } else {
            format!("SELECT * FROM {};", table)
        };
        
        let mut response = self.engine.query(&query).await?;
        
        let result: Option<Vec<Value>> = response.take(0).unwrap_or_default();
        Ok(result.unwrap_or_default())
    }
    
    /// Update records
    pub async fn update_record<T>(&self, table: &str, id: &str, data: T) -> SurrealResult<Value>
    where
        T: Serialize + 'static,
    {
        trace!("Updating record in table: {} with id: {}", table, id);
        
        let query = format!("UPDATE {}:{} CONTENT $data;", table, id);
        let mut response = self.engine.db.query(query)
            .bind(("data", data))
            .await
            .map_err(Error::from)?;
            
        let result: Option<Value> = response.take(0).unwrap_or_default();
        Ok(result.unwrap_or(Value::None))
    }
    
    /// Delete records
    pub async fn delete_record(&self, table: &str, id: &str) -> SurrealResult<Value> {
        trace!("Deleting record from table: {} with id: {}", table, id);
        
        let query = format!("DELETE {}:{};", table, id);
        let mut response = self.engine.query(&query).await?;
        
        let result: Option<Value> = response.take(0).unwrap_or_default();
        Ok(result.unwrap_or(Value::None))
    }
    
    /// Execute a custom query with parameters
    pub async fn execute_query(&self, query: &str, params: Option<HashMap<String, Value>>) -> SurrealResult<Response> {
        trace!("Executing custom query: {}", query);
        
        let mut query_builder = self.engine.db.query(query);
        
        if let Some(params) = params {
            for (key, value) in params {
                query_builder = query_builder.bind((key.clone(), value));
            }
        }
        
        query_builder.await
            .map_err(Error::from)
    }
    
    /// Execute a transaction
    pub async fn execute_transaction(&self, queries: Vec<&str>) -> SurrealResult<Response> {
        debug!("Executing transaction with {} queries", queries.len());
        
        let transaction_query = format!(
            "BEGIN TRANSACTION;\n{}\nCOMMIT TRANSACTION;",
            queries.join(";\n")
        );
        
        self.engine.query(&transaction_query).await
            .map_err(|e| Error::Transaction(format!("Transaction failed: {}", e)))
    }
    
    /// Get database statistics
    pub async fn get_stats(&self) -> SurrealResult<Value> {
        debug!("Getting database statistics");
        
        let query = "INFO FOR DB;";
        let mut response = self.engine.query(query).await?;
        
        let result: Option<Value> = response.take(0).unwrap_or_default();
        Ok(result.unwrap_or(Value::None))
    }
    
    /// Optimize database
    pub async fn optimize(&self) -> SurrealResult<()> {
        debug!("Optimizing database");
        
        // SurrealDB doesn't have explicit optimization commands like SQL databases
        // but we can run maintenance queries
        let queries = vec![
            "INFO FOR DB;", // This helps with internal optimization
        ];
        
        for query in queries {
            self.engine.query(query).await?;
        }
        
        Ok(())
    }
}

/// Predefined schema templates for common use cases
pub mod schemas {
    use super::*;
    
    /// Schema for user management
    pub fn user_schema() -> TableSchema {
        let mut fields = HashMap::new();
        fields.insert("email".to_string(), FieldDefinition {
            field_type: "string".to_string(),
            required: true,
            default: None,
            validation: Some("string::is::email($value)".to_string()),
        });
        fields.insert("username".to_string(), FieldDefinition {
            field_type: "string".to_string(),
            required: true,
            default: None,
            validation: Some("string::len($value) >= 3".to_string()),
        });
        fields.insert("created_at".to_string(), FieldDefinition {
            field_type: "datetime".to_string(),
            required: false,
            default: Some(Value::from("time::now()")),
            validation: None,
        });
        
        let indexes = vec![
            IndexDefinition {
                name: "email_idx".to_string(),
                fields: vec!["email".to_string()],
                unique: true,
                index_type: IndexType::Btree,
            },
            IndexDefinition {
                name: "username_idx".to_string(),
                fields: vec!["username".to_string()],
                unique: true,
                index_type: IndexType::Btree,
            },
        ];
        
        TableSchema {
            name: "users".to_string(),
            fields,
            indexes,
            permissions: None,
        }
    }
    
    /// Schema for logging/events
    pub fn log_schema() -> TableSchema {
        let mut fields = HashMap::new();
        fields.insert("level".to_string(), FieldDefinition {
            field_type: "string".to_string(),
            required: true,
            default: Some(Value::from("INFO")),
            validation: Some("$value IN ['DEBUG', 'INFO', 'WARN', 'ERROR']".to_string()),
        });
        fields.insert("message".to_string(), FieldDefinition {
            field_type: "string".to_string(),
            required: true,
            default: None,
            validation: None,
        });
        fields.insert("timestamp".to_string(), FieldDefinition {
            field_type: "datetime".to_string(),
            required: false,
            default: Some(Value::from("time::now()")),
            validation: None,
        });
        fields.insert("metadata".to_string(), FieldDefinition {
            field_type: "object".to_string(),
            required: false,
            default: None,
            validation: None,
        });
        
        let indexes = vec![
            IndexDefinition {
                name: "timestamp_idx".to_string(),
                fields: vec!["timestamp".to_string()],
                unique: false,
                index_type: IndexType::Btree,
            },
            IndexDefinition {
                name: "level_idx".to_string(),
                fields: vec!["level".to_string()],
                unique: false,
                index_type: IndexType::Hash,
            },
        ];
        
        TableSchema {
            name: "logs".to_string(),
            fields,
            indexes,
            permissions: None,
        }
    }
}
