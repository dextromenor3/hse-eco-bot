use crate::kb::{Note, NoteId, ProviderError};
use crate::util::UnsafeRc;
use chrono::prelude::*;
use rusqlite::{params, Connection};

pub struct Sink {
    db: UnsafeRc<Connection>,
}

impl Sink {
    /// SAFETY: the caller must uphold the invariants of [`UnsafeRc`].
    pub unsafe fn new<'a>(db: UnsafeRc<Connection>) -> Self {
        Self { db }
    }

    pub fn store<Tz>(
        &self,
        newsletter_name: &str,
        note: Note,
        timestamp: DateTime<Tz>,
    ) -> Result<NoteId, ProviderError>
    where
        Tz: TimeZone,
        <Tz as TimeZone>::Offset: std::fmt::Display,
    {
        let txn = self.db.unchecked_transaction()?;
        txn.prepare("INSERT INTO kb_newsletters(name, content, timestamp) VALUES (?, ?, ?)")?
            .execute(params![
                newsletter_name,
                &note.text.raw_text,
                timestamp.to_rfc3339()
            ])?;
        let id = NoteId::from(txn.last_insert_rowid() as u64);
        txn.commit()?;
        trace!("Commit transaction");
        Ok(id)
    }
}
