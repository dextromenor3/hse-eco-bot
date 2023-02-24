use crate::kb::{ItemRef, Note, ProviderError, Tree};
use crate::message::FormattedText;
use crate::util::UnsafeRc;
use rusqlite::Connection;

fn make_tree() -> (Tree, UnsafeRc<Connection>) {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(include_str!("../bootstrap.sql")).unwrap();
    let rc = unsafe { UnsafeRc::new(db) };
    let tree = unsafe { Tree::new(UnsafeRc::clone(&rc)) };
    (tree, rc)
}

#[test]
fn kb_initially_empty() {
    let tree = make_tree().0;
    let root = tree.root_directory_ref().unwrap();
    let dir = root.read().unwrap();
    assert!(dir.children.is_empty(), "KB not empty: {:?}", &dir.children);
}

#[test]
fn note_create_read_delete_ok() {
    let my_note = Note {
        text: FormattedText {
            raw_text: String::from("This is a test note"),
            entities: None,
        },
    };

    let tree = make_tree().0;
    let root = tree.root_directory_ref().unwrap();
    root.create_note(my_note.clone(), "Test note").unwrap();
    let dir = root.read().unwrap();
    assert_eq!(dir.children.len(), 1);
    assert_eq!(dir.children[0].0, "Test note");
    let note_ref = match dir.children[0].1 {
        ItemRef::Directory(_) => panic!("Created note is a directory"),
        ItemRef::Note(note_ref) => note_ref,
    };
    let note = note_ref.read().unwrap();
    assert_eq!(note, my_note);

    assert_eq!(note_ref.name().unwrap(), "Test note");

    note_ref.delete().unwrap();

    let dir = root.read().unwrap();
    assert!(dir.children.is_empty(), "KB not empty: {:?}", &dir.children);
}

#[test]
fn root_dir_ok() {
    let tree = make_tree().0;
    let root = tree.root_directory_ref().unwrap();
    assert_eq!(root.name().unwrap(), None);
    assert_eq!(root.move_to(root.id()), Err(ProviderError::CannotMoveRoot));
    assert_eq!(root.rename("New root"), Err(ProviderError::CannotRenameRoot));
    assert_eq!(root.delete(), Err(ProviderError::CannotDeleteRoot));
}

#[test]
fn dir_create_rename_delete_ok() {
    let tree = make_tree().0;
    let root = tree.root_directory_ref().unwrap();
    root.create_directory("foo").unwrap();
    let dir = root.read().unwrap();
    assert_eq!(dir.children.len(), 1);
    assert_eq!(dir.children[0].0, "foo");
    let dir_ref = match dir.children[0].1 {
        ItemRef::Directory(dir_ref) => dir_ref,
        ItemRef::Note(_) => panic!("Created directory is a note"),
    };
    assert_eq!(dir_ref.name().unwrap().as_deref(), Some("foo"));

    dir_ref.rename("bar").unwrap();
    assert_eq!(dir_ref.name().unwrap().as_deref(), Some("bar"));
    let dir = root.read().unwrap();
    assert_eq!(dir.children.len(), 1);
    assert_eq!(dir.children[0].0, "bar");

    dir_ref.delete().unwrap();
    let dir = root.read().unwrap();
    assert!(dir.children.is_empty());
}

#[test]
fn moves_renames_ok() {
    let tree = make_tree().0;
    let root = tree.root_directory_ref().unwrap();
    let foo = root.create_directory("foo").unwrap();
    let bar = root.create_directory("bar").unwrap();
    let baz = bar.create_directory("baz").unwrap();
    let aaa = root
        .create_note(
            Note {
                text: FormattedText {
                    raw_text: String::from("AAA"),
                    entities: None,
                },
            },
            "aaa",
        )
        .unwrap();
    let bbb = foo
        .create_note(
            Note {
                text: FormattedText {
                    raw_text: String::from("BBB"),
                    entities: None,
                },
            },
            "bbb",
        )
        .unwrap();

    aaa.rename("Aaa").unwrap();
    baz.rename("quux").unwrap();

    assert_eq!(aaa.name().unwrap(), "Aaa");
    assert_eq!(baz.name().unwrap().as_deref(), Some("quux"));

    assert_eq!(
        root.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "Aaa")
            .count(),
        1,
        "`Aaa` not in root",
    );

    assert_eq!(
        bar.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "quux")
            .count(),
        1,
        "`quux` not in `bar`",
    );

    let quux = baz;

    quux.move_to(root.id()).unwrap();
    assert_eq!(
        root.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "quux")
            .count(),
        1,
        "`quux` not in `root`",
    );
    assert_eq!(
        bar.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "quux")
            .count(),
        0,
        "`quux` in `bar`",
    );

    quux.move_to(bar.id()).unwrap();
    assert_eq!(
        root.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "quux")
            .count(),
        0,
        "`quux` in `root`",
    );
    assert_eq!(
        bar.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "quux")
            .count(),
        1,
        "`quux` not in `bar`",
    );

    bbb.move_to(root.id()).unwrap();
    assert_eq!(
        root.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "bbb")
            .count(),
        1,
        "`bbb` not in root",
    );
    assert_eq!(
        foo.read()
            .unwrap()
            .children
            .into_iter()
            .map(|x| x.0)
            .filter(|x| x == "bbb")
            .count(),
        0,
        "`bbb` in `foo`",
    );

    assert_eq!(bar.move_to(quux.id()), Err(ProviderError::WouldCreateLoop));
}

#[test]
fn name_conflicts_ok() {
    let tree = make_tree().0;
    let root = tree.root_directory_ref().unwrap();
    let foo = root.create_directory("foo").unwrap();
    let bar = root.create_directory("bar").unwrap();
    let baz = bar.create_directory("baz").unwrap();
    let aaa = foo
        .create_note(
            Note {
                text: FormattedText {
                    raw_text: String::from("AAA"),
                    entities: None,
                },
            },
            "aaa",
        )
        .unwrap();
    let _bbb = foo
        .create_note(
            Note {
                text: FormattedText {
                    raw_text: String::from("BBB"),
                    entities: None,
                },
            },
            "bbb",
        )
        .unwrap();
    let ccc = bar
        .create_note(
            Note {
                text: FormattedText {
                    raw_text: String::from("CCC"),
                    entities: None,
                },
            },
            "ccc",
        )
        .unwrap();

    assert_eq!(
        foo.rename("bar"),
        Err(ProviderError::TargetNameAlreadyExists(String::from("bar"))),
    );

    assert_eq!(
        aaa.rename("bbb"),
        Err(ProviderError::TargetNameAlreadyExists(String::from("bbb"))),
    );

    baz.rename("foo").unwrap();
    assert_eq!(
        baz.move_to(root.id()),
        Err(ProviderError::TargetNameAlreadyExists(String::from("foo"))),
    );
    
    ccc.rename("bbb").unwrap();
    assert_eq!(
        ccc.move_to(foo.id()),
        Err(ProviderError::TargetNameAlreadyExists(String::from("bbb"))),
    );
}
