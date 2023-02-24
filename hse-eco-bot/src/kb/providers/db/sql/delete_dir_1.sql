WITH RECURSIVE
    subdirs(id) AS (
        VALUES(?1)
        UNION ALL
        SELECT child_id
            FROM kb_dir_children, subdirs
            WHERE kb_dir_children.parent_id = subdirs.id
    )
DELETE FROM kb_dirs
WHERE id IN subdirs
