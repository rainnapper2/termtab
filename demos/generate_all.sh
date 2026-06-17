#!/bin/bash
set -e

cargo build

echo "Generating Tapes..."

cp demos/song.json demos/1_navigation.json
vhs demos/1_navigation.tape

rm -f demos/2_editing.json
vhs demos/2_editing.tape

rm -f demos/3_undo_redo.json
vhs demos/3_undo_redo.tape

cp demos/song.json demos/4_copy_paste.json
vhs demos/4_copy_paste.tape

cp demos/song.json demos/5_note_mode.json
vhs demos/5_note_mode.tape

cp demos/song.json demos/6_annotations.json
vhs demos/6_annotations.tape

cp demos/song.json demos/7_key_change.json
vhs demos/7_key_change.tape

cp demos/song.json demos/8_dynamic_wrap.json
vhs demos/8_dynamic_wrap.tape

echo "All Done!"
