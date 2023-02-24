WITH RECURSIVE
    subdirs(id) AS (
        VALUES(?1)
        UNION ALL
        SELECT child_id
            FROM kb_dir_children, subdirs
            WHERE kb_dir_children.parent_id = subdirs.id
    ),
    subnotes AS (
        SELECT child_id AS id
            FROM kb_note_children
            WHERE kb_note_children.parent_id IN subdirs
    )
DELETE FROM kb_notes
WHERE id IN subnotes
