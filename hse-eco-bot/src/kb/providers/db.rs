use crate::kb::{
    Directory, DirectoryId, DirectoryRef, ItemRef, Note, NoteId, NoteRef, Provider,
    ProviderContext, ProviderError, ProviderId, ProviderUserContext,
};
use crate::message::FormattedText;
use crate::util::UnsafeRc;
use rusqlite::{params, Connection};
use std::collections::HashMap;

pub struct DbProvider {
    db: UnsafeRc<Connection>,
    id: Option<ProviderId>,
    mount_points: HashMap<DirectoryId, ProviderId>,
}

impl DbProvider {
    /// SAFETY: the caller must uphold the invariants of [`UnsafeRc`].
    pub unsafe fn new(db: UnsafeRc<Connection>) -> Self {
        Self {
            db,
            id: None,
            mount_points: HashMap::new(),
        }
    }
}

#[derive(Default)]
struct FailureMap<ForeignKeyF, UniqueF, EmptyF> {
    foreign_key_f: Option<ForeignKeyF>,
    unique_f: Option<UniqueF>,
    empty_f: Option<EmptyF>,
}

fn wrap_sqlite_error<ForeignKeyF, UniqueF, EmptyF>(
    map: FailureMap<ForeignKeyF, UniqueF, EmptyF>,
) -> impl FnOnce(rusqlite::Error) -> ProviderError
where
    ForeignKeyF: FnOnce() -> ProviderError,
    UniqueF: FnOnce() -> ProviderError,
    EmptyF: FnOnce() -> ProviderError,
{
    |e| {
        if let rusqlite::Error::QueryReturnedNoRows = &e {
            if let Some(f) = map.empty_f {
                return f();
            }
        }

        if let Some(rusqlite::ErrorCode::ConstraintViolation) = e.sqlite_error_code() {
            if let Some(ref err) = e.sqlite_error() {
                const FOREIGN_KEY_VIOLATION: i32 = 787;
                const UNIQUE_VIOLATION: i32 = 2067;
                match err.extended_code {
                    FOREIGN_KEY_VIOLATION => {
                        if let Some(f) = map.foreign_key_f {
                            return f();
                        }
                    }
                    UNIQUE_VIOLATION => {
                        if let Some(f) = map.unique_f {
                            return f();
                        }
                    }
                    _ => (),
                }
            }
        }
        ProviderError::from(e)
    }
}

macro_rules! wrap_fn {
    (?) => {
        Option::<fn() -> ProviderError>::None
    };
    ($f:expr) => {
        Some(|| $f)
    };
}

macro_rules! wrap {
    [fk => $fk:tt, unique => $unique:tt, empty => $empty:tt $(,)?] => {
        wrap_sqlite_error(FailureMap {
            foreign_key_f: wrap_fn!($fk),
            unique_f: wrap_fn!($unique),
            empty_f: wrap_fn!($empty),
        })
    };
}

impl Provider for DbProvider {
    fn name(&self) -> String {
        String::from("db")
    }

    fn id(&self) -> ProviderId {
        self.id.unwrap()
    }

    fn assign_id(&mut self, provider_id: ProviderId) {
        self.id = Some(provider_id);
    }

    fn create_note<'c>(
        &mut self,
        ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        target: DirectoryId,
        note: Note,
        name: &str,
    ) -> Result<NoteRef<'c>, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        // TODO: entity serialization.
        txn.prepare(concat!(
            "INSERT INTO kb_notes(content) VALUES (?);\n",
            "SELECT last_insert_rowid;\n"
        ))?
        .execute(params![&note.text.raw_text])?;
        let note_raw_id = txn.last_insert_rowid() as u64;

        txn.prepare(
            "INSERT INTO kb_note_children(parent_id, child_id, child_name) VALUES (?, ?, ?)",
        )?
        .execute(params![u64::from(target), note_raw_id, name])
        .map_err(wrap![
            fk => (ProviderError::NoSuchDirectory(target)),
            unique => (ProviderError::TargetNameAlreadyExists(name.to_owned())),
            empty => ?,
        ])?;

        txn.commit()?;
        Ok(NoteRef::new(note_raw_id.into(), self.id(), ctx))
    }

    fn create_directory<'c>(
        &mut self,
        ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        target: DirectoryId,
        name: &str,
    ) -> Result<DirectoryRef<'c>, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        txn.prepare(concat!("INSERT INTO kb_dirs VALUES (NULL)\n",))?
            .execute(params![])?;
        let dir_raw_id = txn.last_insert_rowid() as u64;

        txn.prepare(
            "INSERT INTO kb_dir_children(parent_id, child_id, child_name) VALUES (?, ?, ?)",
        )?
        .execute(params![u64::from(target), dir_raw_id, name])
        .map_err(wrap![
            fk => (ProviderError::NoSuchDirectory(target)),
            unique => (ProviderError::TargetNameAlreadyExists(name.to_owned())),
            empty => ?,
        ])?;

        txn.commit()?;
        Ok(DirectoryRef::new(dir_raw_id.into(), self.id(), ctx))
    }

    fn root_directory<'c>(
        &self,
        ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
    ) -> Result<DirectoryRef<'c>, ProviderError> {
        Ok(DirectoryRef::new(0.into(), self.id(), ctx))
    }

    fn read_directory<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Directory<'c>, ProviderError> {
        if let Some(provider_id) = self.mount_points.get(&id) {
            let provider = ctx.provider_map[provider_id].borrow();
            return provider.root_directory(ctx, uctx)?.read(uctx);
        }

        let txn = self.db.unchecked_transaction()?;
        let mut result = Directory {
            children: Vec::new(),
        };

        let mut statement = txn.prepare(concat!(
            "SELECT 0, child_id, child_name FROM kb_note_children\n",
            "    WHERE parent_id = ?1\n",
            "UNION ALL\n",
            "SELECT 1, child_id, child_name FROM kb_dir_children\n",
            "    WHERE parent_id = ?1\n",
            "UNION ALL\n",
            "SELECT 2, NULL, NULL FROM kb_dirs\n",
            "    WHERE id = ?1\n",
        ))?;
        let mut rows = statement.query(params![u64::from(id)])?;
        while let Some(row) = rows.next()? {
            match row.get::<_, u32>(0)? {
                0 => {
                    let id: NoteId = row.get::<_, u64>(1)?.into();
                    let name: String = row.get(2)?;
                    result
                        .children
                        .push((name, ItemRef::Note(NoteRef::new(id, self.id(), ctx))));
                }
                1 => {
                    let id: DirectoryId = row.get::<_, u64>(1)?.into();
                    let name: String = row.get(2)?;
                    result.children.push((
                        name,
                        ItemRef::Directory(DirectoryRef::new(id, self.id(), ctx)),
                    ));
                }
                2 => (),
                _ => unreachable!(),
            }
        }
        Ok(result)
    }

    fn get_directory_parent<'c>(
        &self,
        ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Option<DirectoryRef<'c>>, ProviderError> {
        if u64::from(id) == 0 {
            return Ok(None);
        }

        let txn = self.db.unchecked_transaction()?;
        let parent: DirectoryId = txn
            .prepare("SELECT parent_id FROM kb_dir_children WHERE child_id = ?")?
            .query_row(params![u64::from(id)], |row| Ok(u64::into(row.get(0)?)))
            .map_err(wrap![
                fk => ?,
                unique => ?,
                empty => (ProviderError::NoSuchDirectory(id)),
            ])?;
        Ok(Some(DirectoryRef {
            id: parent,
            provider_id: self.id(),
            ctx,
        }))
    }

    fn get_note_parent<'c>(
        &self,
        ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<DirectoryRef<'c>, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let parent: DirectoryId = txn
            .prepare("SELECT parent_id FROM kb_note_children WHERE child_id = ?")?
            .query_row(params![u64::from(id)], |row| Ok(u64::into(row.get(0)?)))
            .map_err(wrap![
                fk => ?,
                unique => ?,
                empty => (ProviderError::NoSuchNote(id)),
            ])?;
        Ok(DirectoryRef {
            id: parent,
            provider_id: self.id(),
            ctx,
        })
    }

    fn get_directory_name<'c>(
        &self,
        _ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Option<String>, ProviderError> {
        if u64::from(id) == 0 {
            return Ok(None);
        }

        let txn = self.db.unchecked_transaction()?;
        let parent_name = txn
            .prepare("SELECT child_name FROM kb_dir_children WHERE child_id = ?")?
            .query_row(params![u64::from(id)], |row| row.get(0))
            .map_err(wrap![
                fk => ?,
                unique => ?,
                empty => (ProviderError::NoSuchDirectory(id)),
            ])?;
        Ok(Some(parent_name))
    }

    fn get_note_name<'c>(
        &self,
        _ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<String, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let parent_name = txn
            .prepare("SELECT child_name FROM kb_note_children WHERE child_id = ?")?
            .query_row(params![u64::from(id)], |row| row.get(0))
            .map_err(wrap![
                fk => ?,
                unique => ?,
                empty => (ProviderError::NoSuchNote(id)),
            ])?;
        Ok(parent_name)
    }

    fn read_note(
        &self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<Note, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let note_text = txn
            .prepare("SELECT content FROM kb_notes WHERE id = ?")?
            .query_row(params![u64::from(id)], |row| row.get(0))
            .map_err(wrap![
                fk => ?,
                unique => ?,
                empty => (ProviderError::NoSuchNote(id)),
            ])?;
        Ok(Note {
            text: FormattedText {
                raw_text: note_text,
                entities: None,
            },
        })
    }

    fn update_note(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        id: NoteId,
        note: Note,
    ) -> Result<(), ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let num_rows_affected = txn
            .prepare("UPDATE kb_notes SET content = ? WHERE id = ?")?
            .execute(params![note.text.raw_text, u64::from(id)])?;
        match num_rows_affected {
            0 => Err(ProviderError::NoSuchNote(id)),
            1 => {
                txn.commit()?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn delete_note(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<(), ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let num_rows_affected = txn
            .prepare("DELETE FROM kb_notes WHERE id = ?")?
            .execute(params![u64::from(id)])?;
        match num_rows_affected {
            0 => Err(ProviderError::NoSuchNote(id)),
            1 => {
                txn.commit()?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn delete_directory(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<(), ProviderError> {
        if id == DirectoryId(0) {
            return Err(ProviderError::CannotDeleteRoot);
        }
        if self.mount_points.contains_key(&id) {
            return Err(ProviderError::OperationNotSupported);
        }
        let txn = self.db.unchecked_transaction()?;
        let num_dirs_affected = txn
            .prepare(include_str!("db/sql/delete_dir_1.sql"))?
            .execute(params![u64::from(id)])?;
        match num_dirs_affected {
            0 => return Err(ProviderError::NoSuchDirectory(id)),
            _ => (),
        }
        txn.prepare(include_str!("db/sql/delete_dir_2.sql"))?
            .execute(params![u64::from(id)])?;
        txn.commit()?;
        Ok(())
    }

    fn rename_directory(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        id: DirectoryId,
        new_name: &str,
    ) -> Result<(), ProviderError> {
        if id == DirectoryId::from(0) {
            return Err(ProviderError::CannotRenameRoot);
        }
        if self.mount_points.contains_key(&id) {
            return Err(ProviderError::OperationNotSupported);
        }
        let txn = self.db.unchecked_transaction()?;
        let num_rows_affected = txn
            .prepare("UPDATE kb_dir_children SET child_name = ?1 WHERE child_id = ?2")?
            .execute(params![new_name, u64::from(id)])
            .map_err(wrap![
                fk => ?,
                unique => (ProviderError::TargetNameAlreadyExists(new_name.to_owned())),
                empty => ?,
            ])?;
        match num_rows_affected {
            0 => Err(ProviderError::NoSuchDirectory(id)),
            1 => {
                txn.commit()?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn rename_note(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        id: NoteId,
        new_name: &str,
    ) -> Result<(), ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let num_rows_affected = txn
            .prepare("UPDATE kb_note_children SET child_name = ?1 WHERE child_id = ?2")?
            .execute(params![new_name, u64::from(id)])
            .map_err(wrap![
                fk => ?,
                unique => (ProviderError::TargetNameAlreadyExists(new_name.to_owned())),
                empty => ?,
            ])?;
        match num_rows_affected {
            0 => Err(ProviderError::NoSuchNote(id)),
            1 => {
                txn.commit()?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn move_directory(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: DirectoryId,
        destination: DirectoryId,
    ) -> Result<(), ProviderError> {
        if id == DirectoryId::from(0) {
            return Err(ProviderError::CannotMoveRoot);
        }
        if self.mount_points.contains_key(&id) {
            return Err(ProviderError::OperationNotSupported);
        }
        let name = self.get_directory_name(ctx, uctx, id)?.unwrap();

        // Immediate transaction is needed because we need to ensure no writes
        // occur between the `check ancestors` read operation and `move directory` write operation.
        let txn = rusqlite::Transaction::new_unchecked(
            &self.db,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        let would_create_loop = txn
            .prepare(include_str!("db/sql/check_ancestors.sql"))?
            .query_row(params![u64::from(destination), u64::from(id)], |row| {
                let num_matches: u64 = row.get(0)?;
                Ok(match num_matches {
                    0 => false,
                    1 => true,
                    _ => unreachable!(),
                })
            })?;
        if would_create_loop {
            return Err(ProviderError::WouldCreateLoop);
        }
        let num_rows_affected = txn
            .prepare("UPDATE kb_dir_children SET parent_id = ?1 WHERE child_id = ?2")?
            .execute(params![u64::from(destination), u64::from(id)])
            .map_err(wrap![
                fk => ?,
                unique => (ProviderError::TargetNameAlreadyExists(name)),
                empty => ?,
            ])?;
        match num_rows_affected {
            0 => Err(ProviderError::NoSuchDirectory(id)),
            1 => {
                txn.commit()?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn move_note(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: NoteId,
        destination: DirectoryId,
    ) -> Result<(), ProviderError> {
        let name = self.get_note_name(ctx, uctx, id)?;
        let txn = self.db.unchecked_transaction()?;
        let num_rows_affected = txn
            .prepare("UPDATE kb_note_children SET parent_id = ?1 WHERE child_id = ?2")?
            .execute(params![u64::from(destination), u64::from(id)])
            .map_err(wrap![
                fk => ?,
                unique => (ProviderError::TargetNameAlreadyExists(name)),
                empty => ?,
            ])?;
        match num_rows_affected {
            0 => Err(ProviderError::NoSuchNote(id)),
            1 => {
                txn.commit()?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn add_mount_point(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        mount_dir: DirectoryId,
        provider: ProviderId,
    ) -> Result<(), ProviderError> {
        self.mount_points.insert(mount_dir, provider);
        Ok(())
    }
}
