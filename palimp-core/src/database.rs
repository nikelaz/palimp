use rusqlite::{Connection, Result};
use std::error::Error;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Database, Box<dyn Error>> {
        let conn = Connection::open(path)?;

        // Enable foreign key constraints
        conn.execute("PRAGMA foreign_keys = ON;", [])?;

        Ok(Database { conn: conn })
    }

    pub fn seed(&self) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sites (
                id INTEGER PRIMARY KEY,
                domain TEXT NOT NULL,
                sitemap_url TEXT
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS crawls (
                id INTEGER PRIMARY KEY,
                site_id INTEGER NOT NULL,
                started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (site_id) REFERENCES sites (id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY,
                crawl_id INTEGER NOT NULL,
                url TEXT NOT NULL,
                final_url TEXT NOT NULL,
                html_content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (crawl_id) REFERENCES crawls (id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS queries (
                id INTEGER PRIMARY KEY,
                crawl_id INTEGER NOT NULL,
                selector TEXT NOT NULL,
                FOREIGN KEY (crawl_id) REFERENCES crawls (id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS results (
                id INTEGER PRIMARY KEY,
                page_id INTEGER NOT NULL,
                selector TEXT NOT NULL,
                count INTEGER NOT NULL,
                FOREIGN KEY (page_id) REFERENCES pages (id) ON DELETE CASCADE
            )",
            [],
        )?;

        println!("Database schema initialized");

        Ok(())
    }
}
