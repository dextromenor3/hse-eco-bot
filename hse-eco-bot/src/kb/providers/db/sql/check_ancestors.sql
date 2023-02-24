WITH RECURSIVE
    ancestors(id) AS (
        VALUES(?1)
        UNION ALL
        SELECT parent_id
            FROM kb_dir_children, ancestors
            WHERE kb_dir_children.child_id = ancestors.id
    )
SELECT COUNT(*)
    FROM ancestors
    WHERE id = ?2
