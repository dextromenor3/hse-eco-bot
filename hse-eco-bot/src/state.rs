pub mod states {
    use crate::db::{FullDirectoryId, FullNoteId};
    use crate::ui::form::{Form, FormInput, FormFillingState};
    use tokio::sync::mpsc::Sender;

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbNavigation {
        pub id: FullDirectoryId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbNoteViewing {
        pub id: FullNoteId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbNoteDeletionConfirmation {
        pub id: FullNoteId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbNoteRenaming {
        pub id: FullNoteId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbNoteCreation {
        pub destination: FullDirectoryId,
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct KbNoteCreationNamed {
        pub destination: FullDirectoryId,
        pub name: String,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbDirectoryEditing {
        pub id: FullDirectoryId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbNoteEditing {
        pub id: FullNoteId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbNoteMovement {
        pub destination: FullDirectoryId,
        pub note: FullNoteId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbDirectoryMovement {
        pub destination: FullDirectoryId,
        pub directory: FullDirectoryId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbDirectoryCreation {
        pub destination: FullDirectoryId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbDirectoryRenaming {
        pub id: FullDirectoryId,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct KbDirectoryDeletion {
        pub id: FullDirectoryId,
    }

    #[derive(Clone)]
    pub struct FormFilling {
        pub form_state: FormFillingState,
        pub return_state: Box<super::DialogState>,
        pub completion_state: Box<super::DialogState>,
        pub on_completion: Sender<(Form, Vec<FormInput>)>,
    }
}

#[derive(Clone)]
pub enum DialogState {
    Initial,
    MainMenu,
    KbNavigation(states::KbNavigation),
    KbNoteViewing(states::KbNoteViewing),
    KbNoteDeletionConfirmation(states::KbNoteDeletionConfirmation),
    KbNoteRenaming(states::KbNoteRenaming),
    KbNoteCreation(states::KbNoteCreation),
    KbNoteCreationNamed(states::KbNoteCreationNamed),
    KbDirectoryEditing(states::KbDirectoryEditing),
    KbNoteEditing(states::KbNoteEditing),
    KbNoteMovement(states::KbNoteMovement),
    KbDirectoryMovement(states::KbDirectoryMovement),
    KbDirectoryCreation(states::KbDirectoryCreation),
    KbDirectoryRenaming(states::KbDirectoryRenaming),
    KbDirectoryDeletion(states::KbDirectoryDeletion),
    FormFilling(states::FormFilling),
    FeedbackTopicSelection,
    SubscriptionsMenu,
}

impl Default for DialogState {
    fn default() -> Self {
        Self::Initial
    }
}
