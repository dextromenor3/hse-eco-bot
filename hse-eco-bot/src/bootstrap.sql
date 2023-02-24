PRAGMA foreign_keys = ON;

BEGIN TRANSACTION;

CREATE TABLE kb_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL
);

CREATE TABLE kb_dirs (
    id INTEGER PRIMARY KEY AUTOINCREMENT
);

CREATE TABLE kb_note_children (
    parent_id INTEGER NOT NULL
        REFERENCES kb_dirs(id) ON DELETE CASCADE,
    child_id INTEGER PRIMARY KEY
        REFERENCES kb_notes(id) ON DELETE CASCADE,
    child_name TEXT NOT NULL,
    UNIQUE (parent_id, child_name)
);

CREATE TABLE kb_dir_children (
    parent_id INTEGER NOT NULL
        REFERENCES kb_dirs(id) ON DELETE CASCADE,
    child_id INTEGER PRIMARY KEY
        REFERENCES kb_dirs(id) ON DELETE CASCADE,
    child_name TEXT NOT NULL,
    UNIQUE (parent_id, child_name)
);

CREATE TABLE kb_newsletters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE TABLE permissions (
    user TEXT UNIQUE NOT NULL,
    edit_kb BOOL NOT NULL,
    receive_feedback BOOL NOT NULL
);

CREATE UNIQUE INDEX kb_notes_by_id ON kb_notes(id);
CREATE UNIQUE INDEX kb_dirs_by_id ON kb_dirs(id);
CREATE UNIQUE INDEX kb_note_children_by_child_id ON kb_note_children(child_id);
CREATE UNIQUE INDEX kb_dir_children_by_child_id ON kb_dir_children(child_id);

INSERT INTO kb_dirs(id) VALUES (0);

COMMIT;
