#!/usr/bin/env python3

from argparse import ArgumentParser
from pathlib import Path
import os
import sqlite3
import sys
from typing import List
from dataclasses import dataclass

def parse_args():
    ap = ArgumentParser()
    ap.add_argument('--privileged-users', '-p', nargs='*', type=str)
    ap.add_argument(
        '--db-dir', '-d',
        type=str,
        help='Path to the database (will be created)',
        required=True,
    )
    return ap.parse_args()

@dataclass
class Tree:
    path: str
    is_file: bool
    children: List['Tree']

def save_tree(cur, tree, parent):
    if tree.is_file:
        with open(tree.path) as f:
            text = f.read()
        cur.execute('INSERT INTO kb_notes VALUES (NULL, ?)', [text.strip()])
        note_id = cur.lastrowid
        cur.execute('INSERT INTO kb_note_children (parent_id, child_id, child_name) VALUES (?, ?, ?)', [parent, note_id, os.path.basename(tree.path)])
    else:
        cur.execute('INSERT INTO kb_dirs VALUES (NULL)')
        dir_id = cur.lastrowid
        cur.execute('INSERT INTO kb_dir_children (parent_id, child_id, child_name) VALUES (?, ?, ?)', [parent, dir_id, os.path.basename(tree.path)])
        for subtree in tree.children:
            save_tree(cur, subtree, dir_id)

def find_kb_files():
    self_path = Path(sys.argv[0])
    kb_path = self_path.parent.parent / 'kb'
    return find_kb_files_in(kb_path)

def find_kb_files_in(path: Path):
    if path.is_dir():
        return Tree(
            path=str(path),
            is_file=False,
            children=[find_kb_files_in(subpath) for subpath in path.iterdir()],
        )
    else:
        return Tree(path=str(path), is_file=True, children=[])

def main():
    args = parse_args()
    db_path = os.path.join(args.db_dir, 'hse-eco-bot.sqlite')
    if os.path.exists(db_path):
        print(f'Error: {db_path} already exists. Refusing to overwrite database. Remove it manually for a clean installation', file=sys.stderr)
        sys.exit(1)

    with sqlite3.connect(db_path) as db:
        self_path = Path(sys.argv[0])
        script_path = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'hse-eco-bot', 'src', 'bootstrap.sql'))
        print(f'Reading installation script from {script_path}...')
        with open(script_path) as f:
            db.executescript(f.read())
        print('Success')

    print('Populating database with prepared notes...')
    tree = find_kb_files()
    cur = db.cursor()
    root_dir = 0
    assert not tree.is_file
    for subtree in tree.children:
        save_tree(cur, subtree, root_dir)
    print('Success')

    if args.privileged_users is not None:
        for user in args.privileged_users:
            print(f'Granting @{user} with admin privileges')
            cur.execute('INSERT INTO permissions VALUES (?, ?, ?)', [user, True, True])

    db.commit()
    print('All done')

if __name__ == '__main__':
    main()
