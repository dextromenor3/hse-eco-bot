use crate::kb::command::{Command, Context, ErasedCommand, ErasedCommandReturnType};
use crate::kb::{
    DirectoryId, DirectoryRef, ItemRef, Note, NoteId, NoteRef, ProviderError, ProviderId,
    ProviderUserContext, Tree,
};
use crate::newsletter::archive::Sink;
use std::fmt::Display;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{self, JoinHandle};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FullDirectoryId {
    pub provider: ProviderId,
    pub directory: DirectoryId,
}

impl From<DirectoryRef<'_>> for FullDirectoryId {
    fn from(r: DirectoryRef<'_>) -> Self {
        Self {
            provider: r.provider_id(),
            directory: r.id(),
        }
    }
}

impl Display for FullDirectoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.provider, self.directory)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FullNoteId {
    pub provider: ProviderId,
    pub note: NoteId,
}

impl From<NoteRef<'_>> for FullNoteId {
    fn from(r: NoteRef<'_>) -> Self {
        Self {
            provider: r.provider_id(),
            note: r.id(),
        }
    }
}

impl Display for FullNoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.provider, self.note)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum FullItemId {
    Directory(FullDirectoryId),
    Note(FullNoteId),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Directory {
    pub directories: Vec<(String, FullDirectoryId)>,
    pub notes: Vec<(String, FullNoteId)>,
}

struct CommandPackage {
    command: ErasedCommand,
    response_sender: oneshot::Sender<ErasedCommandReturnType>,
}

#[derive(Clone)]
pub struct CommandSender {
    sender: mpsc::Sender<CommandPackage>,
}

impl CommandSender {
    pub async fn send<R, F>(&self, command: Command<R, F>) -> R
    where
        F: FnOnce(&mut Context) -> R + Send,
        R: 'static + Send,
        Command<R, F>: Into<ErasedCommand>,
    {
        let erased_result = self.send_erased(command.into()).await;
        let boxed_result = erased_result
            .downcast()
            .expect("Type mismatch when returning from KB access task");
        *boxed_result
    }

    pub async fn send_erased(&self, command: ErasedCommand) -> ErasedCommandReturnType {
        let (response_sender, response_receiver) = oneshot::channel();
        let pkg = CommandPackage {
            command,
            response_sender,
        };
        if self.sender.send(pkg).await.is_err() {
            // Cannot use `Result::expect` here because there is no meaningful way `CommandPackage`
            // (and thus `SendError<CommandPackage>` could implement `Debug`).
            panic!("Cannot send a command to the KB access task");
        }
        response_receiver
            .await
            .expect("Cannot receive the command result from the KB access task")
    }

    pub async fn root_directory(
        &self,
        uctx: ProviderUserContext,
    ) -> Result<FullDirectoryId, ProviderError> {
        let (provider, directory) = self
            .send(Command::new(|ctx| ctx.tree.root_directory()))
            .await?;
        Ok(FullDirectoryId {
            provider,
            directory,
        })
    }

    pub async fn directory_parent(
        &self,
        uctx: ProviderUserContext,
        directory: FullDirectoryId,
    ) -> Result<Option<FullDirectoryId>, ProviderError> {
        self.send(Command::new(move |ctx| {
            let directory = ctx
                .tree
                .make_directory_ref(directory.provider, directory.directory)?;
            let parent_ref = match directory.parent(uctx)? {
                Some(x) => x,
                None => return Ok(None),
            };
            Ok(Some(parent_ref.into()))
        }))
        .await
    }

    pub async fn note_parent(
        &self,
        uctx: ProviderUserContext,
        note: FullNoteId,
    ) -> Result<FullDirectoryId, ProviderError> {
        self.send(Command::new(move |ctx| {
            let note = ctx.tree.make_note_ref(note.provider, note.note)?;
            Ok(note.parent(uctx)?.into())
        }))
        .await
    }

    pub async fn directory_name(
        &self,
        uctx: ProviderUserContext,
        directory: FullDirectoryId,
    ) -> Result<Option<String>, ProviderError> {
        self.send(Command::new(move |ctx| {
            let directory = ctx
                .tree
                .make_directory_ref(directory.provider, directory.directory)?;
            directory.name(uctx)
        }))
        .await
    }

    pub async fn note_name(
        &self,
        uctx: ProviderUserContext,
        note: FullNoteId,
    ) -> Result<String, ProviderError> {
        self.send(Command::new(move |ctx| {
            let note = ctx.tree.make_note_ref(note.provider, note.note)?;
            note.name(uctx)
        }))
        .await
    }

    pub async fn read_directory(
        &self,
        uctx: ProviderUserContext,
        directory: FullDirectoryId,
    ) -> Result<Directory, ProviderError> {
        self.send(Command::new(move |ctx| {
            let directory = ctx
                .tree
                .make_directory_ref(directory.provider, directory.directory)?;
            let mut result = Directory {
                directories: Vec::new(),
                notes: Vec::new(),
            };
            for (name, item_ref) in directory.read(uctx)?.children {
                match item_ref {
                    ItemRef::Directory(dir) => result.directories.push((name, dir.into())),
                    ItemRef::Note(note) => result.notes.push((name, note.into())),
                }
            }
            Ok(result)
        }))
        .await
    }

    pub async fn create_directory(
        &self,
        uctx: ProviderUserContext,
        destination: FullDirectoryId,
        name: String,
    ) -> Result<FullDirectoryId, ProviderError> {
        self.send(Command::new(move |ctx| {
            let destination = ctx
                .tree
                .make_directory_ref(destination.provider, destination.directory)?;
            let created_ref = destination.create_directory(uctx, &name)?;
            Ok(created_ref.into())
        }))
        .await
    }

    pub async fn rename_directory(
        &self,
        uctx: ProviderUserContext,
        directory: FullDirectoryId,
        new_name: String,
    ) -> Result<(), ProviderError> {
        self.send(Command::new(move |ctx| {
            let directory = ctx
                .tree
                .make_directory_ref(directory.provider, directory.directory)?;
            directory.rename(uctx, &new_name)
        }))
        .await
    }

    pub async fn move_directory(
        &self,
        uctx: ProviderUserContext,
        directory: FullDirectoryId,
        destination: FullDirectoryId,
    ) -> Result<(), ProviderError> {
        self.send(Command::new(move |ctx| {
            if directory.provider != destination.provider {
                return Err(ProviderError::CrossProviderMove);
            }

            let directory = ctx
                .tree
                .make_directory_ref(directory.provider, directory.directory)?;
            directory.move_to(uctx, destination.directory)
        }))
        .await
    }

    pub async fn delete_directory(
        &self,
        uctx: ProviderUserContext,
        directory: FullDirectoryId,
    ) -> Result<(), ProviderError> {
        self.send(Command::new(move |ctx| {
            let directory = ctx
                .tree
                .make_directory_ref(directory.provider, directory.directory)?;
            directory.delete(uctx)
        }))
        .await
    }

    pub async fn read_note(
        &self,
        uctx: ProviderUserContext,
        note: FullNoteId,
    ) -> Result<Note, ProviderError> {
        self.send(Command::new(move |ctx| {
            let note = ctx.tree.make_note_ref(note.provider, note.note)?;
            note.read(uctx, )
        }))
        .await
    }

    pub async fn create_note(
        &self,
        uctx: ProviderUserContext,
        destination: FullDirectoryId,
        name: String,
        note: Note,
    ) -> Result<FullNoteId, ProviderError> {
        self.send(Command::new(move |ctx| {
            let destination = ctx
                .tree
                .make_directory_ref(destination.provider, destination.directory)?;
            let created_ref = destination.create_note(uctx, note, &name)?;
            Ok(created_ref.into())
        }))
        .await
    }

    pub async fn rename_note(
        &self,
        uctx: ProviderUserContext,
        note: FullNoteId,
        new_name: String,
    ) -> Result<(), ProviderError> {
        self.send(Command::new(move |ctx| {
            let note = ctx.tree.make_note_ref(note.provider, note.note)?;
            note.rename(uctx, &new_name)
        }))
        .await
    }

    pub async fn move_note(
        &self,
        uctx: ProviderUserContext,
        note: FullNoteId,
        destination: FullDirectoryId,
    ) -> Result<(), ProviderError> {
        self.send(Command::new(move |ctx| {
            if note.provider != destination.provider {
                return Err(ProviderError::CrossProviderMove);
            }

            let note = ctx.tree.make_note_ref(note.provider, note.note)?;
            note.move_to(uctx, destination.directory)
        }))
        .await
    }

    pub async fn delete_note(
        &self,
        uctx: ProviderUserContext,
        note: FullNoteId,
    ) -> Result<(), ProviderError> {
        self.send(Command::new(move |ctx| {
            let note = ctx.tree.make_note_ref(note.provider, note.note)?;
            note.delete(uctx, )
        }))
        .await
    }

    pub async fn update_note(
        &self,
        uctx: ProviderUserContext,
        note_id: FullNoteId,
        note: Note,
    ) -> Result<(), ProviderError> {
        self.send(Command::new(move |ctx| {
            let note_ref = ctx.tree.make_note_ref(note_id.provider, note_id.note)?;
            note_ref.write(uctx, note)?;
            Ok(())
        }))
        .await
    }
}

pub struct AccessTask {
    receiver: mpsc::Receiver<CommandPackage>,
    context: Context,
}

impl AccessTask {
    pub fn new(tree: Tree, newsletter_sink: Sink) -> (Self, CommandSender) {
        let context = Context {
            tree,
            newsletter_sink,
        };
        let (sender, receiver) = mpsc::channel(1);
        let command_sender = CommandSender { sender };
        (Self { receiver, context }, command_sender)
    }

    fn run_blocking(mut self) {
        loop {
            let command_package = match self.receiver.blocking_recv() {
                Some(value) => value,
                None => break,
            };
            let result = command_package.command.run(&mut self.context);
            command_package
                .response_sender
                .send(result)
                .expect("Cannot send the command result from the KB access task");
        }
    }

    pub fn spawn(self) -> JoinHandle<()> {
        task::spawn_blocking(|| self.run_blocking())
    }
}
