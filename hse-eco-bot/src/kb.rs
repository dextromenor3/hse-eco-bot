pub mod command;
pub mod providers;

use crate::message::FormattedText;
use crate::newsletter::archive::Sink;
use crate::newsletter::Newsletter;
use crate::strings::STRINGS;
use crate::user::Permissions;
use crate::user_facing_error::UserFacingError;
use crate::util::UnsafeRc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// The identificator of a directory local to a [`Provider`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DirectoryId(u64);

/// The identificator of a note local to a [`Provider`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NoteId(u64);

/// The identificator of a provider in a [`Tree`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProviderId(u64);

impl Display for DirectoryId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for NoteId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for ProviderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for DirectoryId {
    fn from(raw: u64) -> Self {
        Self(raw)
    }
}

impl From<DirectoryId> for u64 {
    fn from(id: DirectoryId) -> Self {
        id.0
    }
}

impl From<u64> for NoteId {
    fn from(raw: u64) -> Self {
        Self(raw)
    }
}

impl From<NoteId> for u64 {
    fn from(id: NoteId) -> Self {
        id.0
    }
}

impl From<u64> for ProviderId {
    fn from(raw: u64) -> Self {
        Self(raw)
    }
}

impl From<ProviderId> for u64 {
    fn from(id: ProviderId) -> Self {
        id.0
    }
}

/// The reference to an item in a specific [`Provider`].
#[derive(Debug, Copy, Clone)]
pub enum ItemRef<'c> {
    Directory(DirectoryRef<'c>),
    Note(NoteRef<'c>),
}

impl ItemRef<'_> {
    pub fn is_note(&self) -> bool {
        if let Self::Note(_) = self {
            true
        } else {
            false
        }
    }
}

impl<'c> From<DirectoryRef<'c>> for ItemRef<'c> {
    fn from(inner: DirectoryRef<'c>) -> Self {
        Self::Directory(inner)
    }
}

impl<'c> From<NoteRef<'c>> for ItemRef<'c> {
    fn from(inner: NoteRef<'c>) -> Self {
        Self::Note(inner)
    }
}

/// The reference to an directory in a specific [`Provider`].
#[derive(Copy, Clone)]
pub struct DirectoryRef<'c> {
    id: DirectoryId,
    provider_id: ProviderId,
    ctx: ProviderContext<'c>,
}

impl std::fmt::Debug for DirectoryRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DirectoryRef")
            .field("id", &self.id)
            .field("provider_id", &self.provider_id)
            .finish()
    }
}

impl<'c> DirectoryRef<'c> {
    /// Create a [`DirectoryRef`].
    pub fn new(id: DirectoryId, provider_id: ProviderId, ctx: ProviderContext<'c>) -> Self {
        Self {
            id,
            provider_id,
            ctx,
        }
    }

    /// Get the directory ID.
    pub fn id(&self) -> DirectoryId {
        self.id
    }

    /// Get the provider ID of this directory.
    pub fn provider_id(&self) -> ProviderId {
        self.provider_id
    }

    /// Get the provider of this directory.
    pub fn provider(&self) -> &'c RefCell<dyn Provider + Send> {
        self.ctx.provider_map[&self.provider_id].as_ref()
    }

    /// List the items in this directory.
    pub fn read(&self, uctx: ProviderUserContext) -> Result<Directory<'c>, ProviderError> {
        self.provider()
            .borrow()
            .read_directory(self.ctx, uctx, self.id)
    }

    /// Get the parent directory.
    pub fn parent(
        &self,
        uctx: ProviderUserContext,
    ) -> Result<Option<DirectoryRef<'c>>, ProviderError> {
        self.provider()
            .borrow()
            .get_directory_parent(self.ctx, uctx, self.id)
    }

    /// Create a note in this directory.
    pub fn create_note(
        &self,
        uctx: ProviderUserContext,
        note: Note,
        name: &str,
    ) -> Result<NoteRef<'c>, ProviderError> {
        self.provider()
            .borrow_mut()
            .create_note(self.ctx, uctx, self.id, note, name)
    }

    /// Create a subdirectory in this directory.
    pub fn create_directory(
        &self,
        uctx: ProviderUserContext,
        name: &str,
    ) -> Result<DirectoryRef<'c>, ProviderError> {
        self.provider()
            .borrow_mut()
            .create_directory(self.ctx, uctx, self.id, name)
    }

    /// Create a mountpoint of another provider in this directory.
    pub fn mount_here(
        &self,
        uctx: ProviderUserContext,
        provider_id: ProviderId,
    ) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .add_mount_point(self.ctx, uctx, self.id, provider_id)
    }

    /// Rename this directory.
    pub fn rename(&self, uctx: ProviderUserContext, new_name: &str) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .rename_directory(self.ctx, uctx, self.id, new_name)
    }

    /// Move this directory elsewhere.
    pub fn move_to(
        &self,
        uctx: ProviderUserContext,
        destination: DirectoryId,
    ) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .move_directory(self.ctx, uctx, self.id, destination)
    }

    /// Delete this directory recursively.
    pub fn delete(&self, uctx: ProviderUserContext) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .delete_directory(self.ctx, uctx, self.id)
    }

    /// Get the name of this directory if it is not the root directory.
    pub fn name(&self, uctx: ProviderUserContext) -> Result<Option<String>, ProviderError> {
        self.provider()
            .borrow()
            .get_directory_name(self.ctx, uctx, self.id)
    }
}

/// The reference to a note in a specific [`Provider`].
#[derive(Copy, Clone)]
pub struct NoteRef<'c> {
    id: NoteId,
    provider_id: ProviderId,
    ctx: ProviderContext<'c>,
}

impl std::fmt::Debug for NoteRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoteRef")
            .field("id", &self.id)
            .field("provider_id", &self.provider_id)
            .finish()
    }
}

impl<'c> NoteRef<'c> {
    /// Create a [`NoteRef`].
    pub fn new(id: NoteId, provider_id: ProviderId, ctx: ProviderContext<'c>) -> Self {
        Self {
            id,
            provider_id,
            ctx,
        }
    }
    /// Get the note ID.
    pub fn id(&self) -> NoteId {
        self.id
    }

    /// Get the provider ID of this note.
    pub fn provider_id(&self) -> ProviderId {
        self.provider_id
    }

    /// Get the provider of this note.
    pub fn provider(&self) -> &'c RefCell<dyn Provider + Send> {
        self.ctx.provider_map[&self.provider_id].as_ref()
    }

    /// Get the parent directory of this note.
    pub fn parent(&self, uctx: ProviderUserContext) -> Result<DirectoryRef<'c>, ProviderError> {
        self.provider()
            .borrow()
            .get_note_parent(self.ctx, uctx, self.id)
    }

    /// Read this note.
    pub fn read(&self, uctx: ProviderUserContext) -> Result<Note, ProviderError> {
        self.provider().borrow().read_note(self.ctx, uctx, self.id)
    }

    /// Update this note.
    pub fn write(&self, uctx: ProviderUserContext, new_note: Note) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .update_note(self.ctx, uctx, self.id, new_note)
    }

    /// Rename this note.
    pub fn rename(&self, uctx: ProviderUserContext, new_name: &str) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .rename_note(self.ctx, uctx, self.id, new_name)
    }

    /// Move this note elsewhere.
    pub fn move_to(
        &self,
        uctx: ProviderUserContext,
        destination: DirectoryId,
    ) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .move_note(self.ctx, uctx, self.id, destination)
    }

    /// Delete this note.
    pub fn delete(&self, uctx: ProviderUserContext) -> Result<(), ProviderError> {
        self.provider()
            .borrow_mut()
            .delete_note(self.ctx, uctx, self.id)
    }

    /// Get the name of this note.
    pub fn name(&self, uctx: ProviderUserContext) -> Result<String, ProviderError> {
        self.provider()
            .borrow()
            .get_note_name(self.ctx, uctx, self.id)
    }
}

/// The data of a note.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Note {
    /// The text of the note.
    pub text: FormattedText,
}

/// The data of a directory.
#[derive(Clone)]
pub struct Directory<'c> {
    /// The names of and references to sub-items in the directory.
    pub children: Vec<(String, ItemRef<'c>)>,
}

/// The error returned by a [`Provider`] if some of its operations fail.
#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum ProviderError {
    /// The directory with the provided ID does not exist.
    NoSuchDirectory(DirectoryId),
    /// The directory with the provided ID does not exist.
    NoSuchNote(NoteId),
    /// The operation would create a directory loop.
    WouldCreateLoop,
    /// The operation is not supported by this provider.
    OperationNotSupported,
    /// Renaming of the global root directory was requested, which is not allowed.
    CannotRenameRoot,
    /// Moving of the global root directory was requested, which is not allowed.
    CannotMoveRoot,
    /// Deletion of the global root directory was requested, which is not allowed.
    CannotDeleteRoot,
    /// There is already a resource with the requested name.
    TargetNameAlreadyExists(String),
    /// The provider with such ID does not exist.
    NoSuchProvider(ProviderId),
    /// Moving an item between providers is not supported.
    CrossProviderMove,
    /// SQLite error.
    SqliteError(rusqlite::Error),
    /// Storage is corrupt.
    Corrupt { description: String },
    /// Permission denied.
    PermissionDenied,
}

impl Display for ProviderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchDirectory(id) => write!(f, "There is no directory with ID {}", id),
            Self::NoSuchNote(id) => write!(f, "There is no note with ID {}", id),
            Self::WouldCreateLoop => write!(f, "Operation would create a directory loop"),
            Self::OperationNotSupported => write!(f, "Operation is not supported"),
            Self::CannotRenameRoot => write!(f, "Cannot rename the root directory"),
            Self::CannotMoveRoot => write!(f, "Cannot move the root directory"),
            Self::CannotDeleteRoot => write!(f, "Cannot delete the root directory"),
            Self::TargetNameAlreadyExists(ref name) => {
                write!(f, "Target name already exists: {}", name)
            }
            Self::NoSuchProvider(id) => {
                write!(f, "Provider with ID {} does not exist", id)
            }
            Self::CrossProviderMove => write!(f, "Cannot move an item between providers"),
            Self::SqliteError(e) => write!(f, "SQLite error: {}", e),
            Self::Corrupt { description } => write!(f, "Database is corrupt: {}", description),
            Self::PermissionDenied => write!(f, "Permission denied"),
        }
    }
}

impl Error for ProviderError {}

impl UserFacingError for ProviderError {
    fn user_message(&self) -> FormattedText {
        let p = &STRINGS.errors.provider;
        match self {
            Self::NoSuchDirectory(_id) => p.no_such_directory(),
            Self::NoSuchNote(_id) => p.no_such_note(),
            Self::WouldCreateLoop => p.would_create_loop(),
            Self::OperationNotSupported => p.operation_not_supported(),
            Self::CannotRenameRoot => p.cannot_rename_root(),
            Self::CannotMoveRoot => p.cannot_move_root(),
            Self::CannotDeleteRoot => p.cannot_delete_root(),
            Self::TargetNameAlreadyExists(ref name) => p.target_name_already_exists(name),
            Self::NoSuchProvider(_id) => STRINGS.errors.kb.no_such_provider(),
            Self::CrossProviderMove => p.cross_provider_move(),
            Self::SqliteError(_) => p.internal_error(),
            Self::Corrupt { .. } => p.internal_error(),
            Self::PermissionDenied => p.permission_denied(),
        }
    }
}

impl From<rusqlite::Error> for ProviderError {
    fn from(e: rusqlite::Error) -> Self {
        Self::SqliteError(e)
    }
}

/// The context each provider is provided with for its operations.
#[derive(Copy, Clone)]
pub struct ProviderContext<'c> {
    /// The mapping that allows to get a provider by its ID.
    pub provider_map: &'c HashMap<ProviderId, Box<RefCell<dyn Provider + Send>>>,
    pub newsletters: &'c HashMap<String, Box<dyn Fn(&Permissions) -> bool + Send + Sync>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ProviderUserContext {
    pub permissions: Permissions,
}

/// A provider and/or a storage of a subtree of directories and notes.
pub trait Provider {
    /// Get provider name.
    fn name(&self) -> String;

    /// Create a new note in the `target` directory.
    fn create_note<'c>(
        &mut self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        target: DirectoryId,
        note: Note,
        name: &str,
    ) -> Result<NoteRef<'c>, ProviderError>;

    /// Create a new directory in the `target` directory.
    fn create_directory<'c>(
        &mut self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        target: DirectoryId,
        name: &str,
    ) -> Result<DirectoryRef<'c>, ProviderError>;

    /// Get the root directory of this provider.
    fn root_directory<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
    ) -> Result<DirectoryRef<'c>, ProviderError>;

    /// List items in a directory.
    fn read_directory<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Directory<'c>, ProviderError>;

    /// Get the parent directory of the directory with a given ID.
    ///
    /// If the specified directory is the root directory for this provider but not on the whole,
    /// the corresponding directory of the respective provider (on top of which this one is
    /// mounted) must be returned. If the current directory is the global root, `None` must be returned.
    fn get_directory_parent<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Option<DirectoryRef<'c>>, ProviderError>;

    /// Get the parent directory of a note.
    fn get_note_parent<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<DirectoryRef<'c>, ProviderError>;

    /// Get the name of a directory, if it is not the root directory.
    fn get_directory_name<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<Option<String>, ProviderError> {
        let parent_ref = match self.get_directory_parent(ctx, uctx, id)? {
            Some(x) => x,
            None => return Ok(None),
        };
        let children = parent_ref.read(uctx)?.children;
        Ok(Some(
            children
                .into_iter()
                .find_map(|(name, value)| match value {
                    ItemRef::Directory(dir_ref) => {
                        if dir_ref.id == id && dir_ref.provider_id == self.id() {
                            Some(name)
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .expect("Broken invariant: child dir not found in parent dir"),
        ))
    }

    /// Get the name of a note.
    fn get_note_name<'c>(
        &self,
        ctx: ProviderContext<'c>,
        uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<String, ProviderError> {
        let parent_ref = self.get_note_parent(ctx, uctx, id)?;
        let children = parent_ref.read(uctx)?.children;
        Ok(children
            .into_iter()
            .find_map(|(name, value)| match value {
                ItemRef::Note(note_ref) => {
                    if note_ref.id == id && note_ref.provider_id == self.id() {
                        Some(name)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .expect("Broken invariant: child note not found in parent dir"))
    }

    /// Read a note by its ID.
    fn read_note(
        &self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<Note, ProviderError>;

    /// Update a note.
    ///
    /// If no note with such ID exists, an error should be returned.
    fn update_note(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: NoteId,
        note: Note,
    ) -> Result<(), ProviderError>;

    /// Delete a note.
    fn delete_note(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: NoteId,
    ) -> Result<(), ProviderError>;

    /// Delete a directory recursively.
    fn delete_directory(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: DirectoryId,
    ) -> Result<(), ProviderError>;

    /// Rename a directory.
    ///
    /// The root directory of the current provider cannot be renamed. This implies that mount point
    /// names cannot be changed. This is an acceptable restriction, because for this project the
    /// ability to rename mount points is not needed.
    fn rename_directory(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: DirectoryId,
        new_name: &str,
    ) -> Result<(), ProviderError>;

    /// Rename a note.
    fn rename_note(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: NoteId,
        new_name: &str,
    ) -> Result<(), ProviderError>;

    /// Move a directory within the provider tree.
    fn move_directory(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: DirectoryId,
        destination: DirectoryId,
    ) -> Result<(), ProviderError>;

    /// Move a note within the provider tree.
    fn move_note(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        id: NoteId,
        destination: DirectoryId,
    ) -> Result<(), ProviderError>;

    /// Mount another provider in the specified directory.
    fn add_mount_point(
        &mut self,
        ctx: ProviderContext<'_>,
        uctx: ProviderUserContext,
        mount_dir: DirectoryId,
        provider: ProviderId,
    ) -> Result<(), ProviderError>;

    /// Get this provider's ID.
    ///
    /// May panic before the ID is first assigned.
    fn id(&self) -> ProviderId;

    /// Assign an ID to this provider.
    fn assign_id(&mut self, provider_id: ProviderId);
}

/// The global tree of knowledge base items.
pub struct Tree {
    providers: HashMap<ProviderId, Box<RefCell<dyn Provider + Send>>>,
    root_provider: ProviderId,
    newsletters: HashMap<String, Box<dyn Fn(&Permissions) -> bool + Send + Sync>>,
}

impl Tree {
    /// Create an example of a tree.
    ///
    /// This method is temporary and its signature is subject to change.
    ///
    /// SAFETY: the caller must uphold the invariants of [`UnsafeRc`].
    pub unsafe fn new<'a>(
        db: UnsafeRc<rusqlite::Connection>,
        newsletters: &[&'a dyn Newsletter],
    ) -> (Self, HashMap<String, ProviderId>, Sink) {
        let mut providers = HashMap::new();

        let mut root_provider: Box<RefCell<dyn Provider + Send>> = Box::new(RefCell::new(
            providers::db::DbProvider::new(UnsafeRc::clone(&db)),
        ));
        let root_provider_id = ProviderId::from(0);
        root_provider.get_mut().assign_id(root_provider_id);
        providers.insert(root_provider_id, root_provider);

        let ctx_newsletters = newsletters
            .iter()
            .copied()
            .map(|nl| (nl.name(), nl.allowed()))
            .collect();

        let uctx = ProviderUserContext {
            permissions: Permissions::all(),
        };

        let mount_point_id = {
            let ctx = ProviderContext {
                provider_map: &providers,
                newsletters: &ctx_newsletters,
            };
            let root_dir = providers[&root_provider_id]
                .borrow()
                .root_directory(
                    ctx,
                    ProviderUserContext {
                        permissions: Permissions::all(),
                    },
                )
                .unwrap();
            const ARCHIVE_DIR: &str = "Архив рассылок";
            root_dir
                .create_directory(
                    ProviderUserContext {
                        permissions: Permissions::all(),
                    },
                    ARCHIVE_DIR,
                )
                .unwrap_or_else(|_| {
                    let item_ref = root_dir
                        .read(uctx)
                        .unwrap()
                        .children
                        .iter()
                        .find(|&(name, _)| name == ARCHIVE_DIR)
                        .unwrap()
                        .1;
                    match item_ref {
                        ItemRef::Directory(d) => d,
                        _ => unreachable!(),
                    }
                })
                .id()
        };

        let mut archive_provider: Box<RefCell<dyn Provider + Send>> =
            Box::new(RefCell::new(providers::archive::ArchiveProvider::new(
                UnsafeRc::clone(&db),
                newsletters.iter().copied(),
                (root_provider_id, mount_point_id),
            )));
        let archive_provider_id = ProviderId::from(1);
        archive_provider.get_mut().assign_id(archive_provider_id);
        providers.insert(archive_provider_id, archive_provider);

        {
            let ctx = ProviderContext {
                provider_map: &providers,
                newsletters: &ctx_newsletters,
            };
            providers[&root_provider_id]
                .borrow_mut()
                .add_mount_point(
                    ctx,
                    ProviderUserContext {
                        permissions: Permissions::all(),
                    },
                    mount_point_id,
                    archive_provider_id,
                )
                .unwrap();
        }

        let provider_registry = providers
            .iter()
            .map(|(&id, provider)| (provider.borrow().name(), id))
            .collect();

        let root_provider = ProviderId::from(0);
        let me = Self {
            providers,
            root_provider,
            newsletters: ctx_newsletters,
        };
        let newsletter_sink = Sink::new(db);
        (me, provider_registry, newsletter_sink)
    }

    /// Get the root provider of this tree.
    ///
    /// Returns both the ID of the provider and a reference to it.
    pub fn root_provider(&self) -> (ProviderId, &RefCell<dyn Provider + Send>) {
        let provider = self
            .providers
            .get(&self.root_provider)
            .expect("Broken invariant: Tree's `root_provider` is not present in this Tree")
            .as_ref();
        (self.root_provider, provider)
    }

    /// Return the root directory and the corresponding provider ID.
    pub fn root_directory(&self) -> Result<(ProviderId, DirectoryId), ProviderError> {
        let (provider_id, provider) = self.root_provider();
        let ctx = ProviderContext {
            provider_map: &self.providers,
            newsletters: &self.newsletters,
        };
        let directory_ref = provider.borrow().root_directory(
            ctx,
            ProviderUserContext {
                permissions: Permissions::all(),
            },
        )?;
        Ok((provider_id, directory_ref.id()))
    }

    /// Return the [`DirectoryRef`] to the root directory.
    pub fn root_directory_ref(&self) -> Result<DirectoryRef<'_>, ProviderError> {
        let (provider_id, directory_id) = self.root_directory()?;
        self.make_directory_ref(provider_id, directory_id)
    }

    /// Given provider and directory IDs, make a corresponding [`DirectoryRef`].
    pub fn make_directory_ref(
        &self,
        provider_id: ProviderId,
        directory_id: DirectoryId,
    ) -> Result<DirectoryRef<'_>, ProviderError> {
        if !self.providers.contains_key(&provider_id) {
            return Err(ProviderError::NoSuchProvider(provider_id));
        }
        let ctx = ProviderContext {
            provider_map: &self.providers,
            newsletters: &self.newsletters,
        };

        Ok(DirectoryRef {
            provider_id,
            ctx,
            id: directory_id,
        })
    }

    /// Given provider and note IDs, make a corresponding [`NoteRef`].
    pub fn make_note_ref(
        &self,
        provider_id: ProviderId,
        note_id: NoteId,
    ) -> Result<NoteRef<'_>, ProviderError> {
        if !self.providers.contains_key(&provider_id) {
            return Err(ProviderError::NoSuchProvider(provider_id));
        }
        let ctx = ProviderContext {
            provider_map: &self.providers,
            newsletters: &self.newsletters,
        };

        Ok(NoteRef {
            provider_id,
            ctx,
            id: note_id,
        })
    }
}
