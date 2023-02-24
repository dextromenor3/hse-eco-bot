use crate::kb::{
    Directory, DirectoryId, DirectoryRef, ItemRef, Note, NoteId, NoteRef, Provider,
    ProviderContext, ProviderError, ProviderId, ProviderUserContext,
};
use crate::message::FormattedText;
use crate::newsletter::Newsletter;
use crate::util::UnsafeRc;
use chrono::prelude::*;
use rusqlite::{params, Connection};
use std::collections::HashMap;

const ROOT_DIR_ID: DirectoryId = DirectoryId(u64::MAX);

pub struct ArchiveProvider {
    db: UnsafeRc<Connection>,
    id: Option<ProviderId>,
    names_map: HashMap<String, (u64, String)>,
    ids_map: HashMap<u64, String>,
    mounted_on: (ProviderId, DirectoryId),
}

impl ArchiveProvider {
    /// SAFETY: the caller must uphold the invariants of [`UnsafeRc`].
    pub unsafe fn new<'a>(
        db: UnsafeRc<Connection>,
        newsletters: impl IntoIterator<Item = &'a dyn Newsletter>,
        mounted_on: (ProviderId, DirectoryId),
    ) -> Self {
        let mut names_map = HashMap::new();
        let mut ids_map = HashMap::new();
        for (i, nl) in newsletters.into_iter().enumerate() {
            names_map.insert(nl.name(), (i as u64, nl.description()));
            ids_map.insert(i as u64, nl.name());
        }
        Self {
            db,
            id: None,
            names_map,
            ids_map,
            mounted_on,
        }
    }
}

fn make_note_name<Tz>(id: u64, timestamp: DateTime<Tz>) -> String
where
    Tz: TimeZone,
    <Tz as TimeZone>::Offset: std::fmt::Display,
{
    format!("{} (№ {})", timestamp.format("%Y-%m-%d %H:%M:%S"), id)
}

impl Provider for ArchiveProvider {
    fn name(&self) -> String {
        String::from("newsletter-archive")
    }

    fn create_note<'c>(
        &mut self,
        _ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        _target: DirectoryId,
        _note: Note,
        _name: &str,
    ) -> Result<NoteRef<'c>, ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn create_directory<'c>(
        &mut self,
        _ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        _target: DirectoryId,
        _name: &str,
    ) -> Result<DirectoryRef<'c>, ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn root_directory<'c>(
        &self,
        ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
    ) -> Result<DirectoryRef<'c>, ProviderError> {
        Ok(DirectoryRef {
            id: ROOT_DIR_ID,
            provider_id: self.id(),
            ctx,
        })
    }

    fn read_directory<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Directory<'c>, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let children = if id == ROOT_DIR_ID {
            self.ids_map
                .iter()
                .map(|(&k, v)| {
                    (
                        v,
                        self.names_map[v].1.clone(),
                        ItemRef::Directory(DirectoryRef::new(k.into(), self.id(), ctx)),
                    )
                })
                .filter(|&(name, _, _)| {
                    ctx.newsletters[name](&uctx.permissions)
                })
                .map(|(_a, b, c)| (b, c))
                .collect()
        } else {
            let name = self
                .ids_map
                .get(&id.into())
                .ok_or(ProviderError::NoSuchDirectory(id))?;

            if !ctx.newsletters[name](&uctx.permissions) {
                return Err(ProviderError::PermissionDenied);
            }

            txn.prepare("SELECT id, timestamp FROM kb_newsletters WHERE name = ?")?
                .query_map(params![name], |row| {
                    let id: u64 = row.get(0)?;
                    let timestamp_str: String = row.get(1)?;
                    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str).unwrap();
                    let note_name = make_note_name(id, timestamp);
                    let item_ref = ItemRef::Note(NoteRef::new(id.into(), self.id(), ctx));
                    Ok((note_name, item_ref))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(Directory { children })
    }

    fn get_directory_parent<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Option<DirectoryRef<'c>>, ProviderError> {
        if id == ROOT_DIR_ID {
            let mount_dir = DirectoryRef::new(self.mounted_on.1, self.mounted_on.0, ctx);
            mount_dir.parent(uctx)
        } else {
            Ok(Some(DirectoryRef::new(ROOT_DIR_ID, self.id(), ctx)))
        }
    }

    fn get_note_parent<'c>(
        &self,
        ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<DirectoryRef<'c>, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let name: String = txn
            .prepare("SELECT name FROM kb_newsletters WHERE id = ?")?
            .query_row(params![id.0], |row| row.get(0))?;
        let dir_id = self.names_map[&name].0.into();
        Ok(DirectoryRef::new(dir_id, self.id(), ctx))
    }

    fn get_directory_name<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Option<String>, ProviderError> {
        if id == ROOT_DIR_ID {
            let (provider_id, directory_id) = self.mounted_on;
            return ctx.provider_map[&provider_id].borrow().get_directory_name(
                ctx,
                uctx,
                directory_id,
            );
        }
        let name = self
            .ids_map
            .get(&id.0)
            .ok_or(ProviderError::NoSuchDirectory(id))?;
        Ok(Some(self.names_map[name].1.clone()))
    }

    fn get_note_name<'c>(
        &self,
        _ctx: ProviderContext<'c>,
        _uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<String, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let timestamp_str: String = txn
            .prepare("SELECT timestamp FROM kb_newsletters WHERE id = ?")?
            .query_row(params![id.0], |row| row.get(0))?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str).unwrap();
        Ok(make_note_name(id.into(), timestamp))
    }

    fn read_note(
        &self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<Note, ProviderError> {
        let txn = self.db.unchecked_transaction()?;
        let (name, content): (String, String) = txn
            .prepare("SELECT name, content FROM kb_newsletters WHERE id = ?")?
            .query_row(params![id.0], |row| Ok((row.get(0)?, row.get(1)?)))?;
        // TODO: entities.
        let note = Note {
            text: FormattedText {
                raw_text: content,
                entities: None,
            },
        };

        if !ctx.newsletters[&name](&uctx.permissions) {
            return Err(ProviderError::PermissionDenied);
        }
        Ok(note)
    }

    fn update_note(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _id: NoteId,
        _note: Note,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn delete_note(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _id: NoteId,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn delete_directory(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _id: DirectoryId,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn rename_directory(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _id: DirectoryId,
        _new_name: &str,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn rename_note(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _id: NoteId,
        _new_name: &str,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn move_directory(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _id: DirectoryId,
        _destination: DirectoryId,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn move_note(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _id: NoteId,
        _destination: DirectoryId,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn add_mount_point(
        &mut self,
        _ctx: ProviderContext<'_>,
        _uctx: ProviderUserContext,
        _mount_dir: DirectoryId,
        _provider: ProviderId,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::OperationNotSupported)
    }

    fn id(&self) -> ProviderId {
        self.id.unwrap()
    }

    fn assign_id(&mut self, provider_id: ProviderId) {
        self.id = Some(provider_id);
    }
}
